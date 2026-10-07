/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

static BOOL nvmtl_helpers_independent(NSData *input) {
    if (!input || input.length < 20 || input.length % 4) return NO;
    const uint32_t *w=input.bytes; size_t count=input.length/4;
    if (w[0]!=0x07230203 || w[3]>0xffffff00u) return NO;
    uint32_t glsl=0,entries=0; BOOL physical=NO;
    for(size_t i=5;i<count;) {
        uint32_t n=w[i]>>16,op=w[i]&65535;
        if(!n || n>count-i) return NO;
        if(op==15) { if(n<4 || n>=0xffff || w[i+1]!=4 || ++entries!=1) return NO; }
        if(op==32 && n==4 && w[i+2]==5349) physical=YES;
        if(op==71 && n>=4 && w[i+2]==11 && w[i+3]==23) return NO;
        if(op==11) {
            if(n!=6 || memcmp(w+i+2,"GLSL.std.450",13)) return NO;
            glsl=w[i+1];
        }
        if(op==12) {
            if(n<5 || !glsl || w[i+3]!=glsl) return NO;
            uint32_t ext=w[i+4];
            if(!((ext>=1 && ext<=75)||(ext>=79 && ext<=81))) return NO;
        } else {
            BOOL safe = op<=8 || op==10 || op==11 || (op>=14 && op<=17)
                || (op>=19 && op<=24) || (op>=28 && op<=33) || op==39
                || (op>=41 && op<=44) || op==46 || (op>=48 && op<=52)
                || (op>=54 && op<=57) || op==59 || (op>=61 && op<=68)
                || (op>=70 && op<=75) || (op>=77 && op<=84)
                || (op>=109 && op<=124) || (op>=126 && op<=152)
                || (op>=154 && op<=191) || (op>=194 && op<=205)
                || (op>=245 && op<=257) || op==317 || op==330;
            if(!safe) return NO;
        }
        i+=n;
    }
    return entries==1 && physical;
}

static NSData *nvmtl_independent_helpers(NSData *input) {
    if (!nvmtl_helpers_independent(input)) return input;
    const uint32_t *w=input.bytes;size_t count=input.length/4;
    uint32_t entry=0,boolean=0,pointer=0,helper=0;
    for(size_t i=5;i<count;){uint32_t n=w[i]>>16,op=w[i]&65535;if(!n||n>count-i){nvlog("fragment helpers: malformed rewrite input; retaining original");return input;}
        if(op==15 && n>=4 && w[i+1]==4)entry=w[i+2];
        if(op==20 && n==2)boolean=w[i+1];
        if(op==71 && n==4 && w[i+2]==11 && w[i+3]==23)helper=w[i+1];
        i+=n;
    }
    if(!entry)return input;
    for(size_t i=5;i<count;i+=w[i]>>16)if((w[i]&65535)==32 && (w[i]>>16)==4 && w[i+2]==1 && w[i+3]==boolean)pointer=w[i+1];
    uint32_t next=w[3];BOOL addbool=!boolean,addptr=!pointer,addhelper=!helper;
    if(addbool)boolean=next++;if(addptr)pointer=next++;if(addhelper)helper=next++;
    uint32_t newentry=next++,loaded=next++,retblock=next++;
    NSMutableData *out=[NSMutableData data];uint32_t header[5];memcpy(header,w,20);header[3]=next;[out appendBytes:header length:20];
#define DIAG_WORDS(...) do{uint32_t words[]={__VA_ARGS__};[out appendBytes:words length:sizeof(words)];}while(0)
    BOOL decorated=NO,declared=NO,inmain=NO,injected=NO;
    for(size_t i=5;i<count;){uint32_t n=w[i]>>16,op=w[i]&65535;
        if(!decorated && op>=19 && op<=39){if(addhelper)DIAG_WORDS((4u<<16)|71,helper,11,23);decorated=YES;}
        if(!declared && op==54){
            if(addbool)DIAG_WORDS((2u<<16)|20,boolean);
            if(addptr)DIAG_WORDS((4u<<16)|32,pointer,1,boolean);
            if(addhelper)DIAG_WORDS((4u<<16)|59,pointer,helper,1);
            declared=YES;
        }
        if(op==54)inmain=w[i+2]==entry;
        if(op==15 && w[i+1]==4 && addhelper){uint32_t head=((n+1)<<16)|15;[out appendBytes:&head length:4];[out appendBytes:w+i+1 length:(n-1)*4];[out appendBytes:&helper length:4];i+=n;continue;}
        if(inmain && !injected && op==248){
            uint32_t oldentry=w[i+1];DIAG_WORDS((2u<<16)|248,newentry);i+=n;
            while(i<count && ((w[i]&65535)==59 || (w[i]&65535)==8 || (w[i]&65535)==317)){uint32_t k=w[i]>>16;[out appendBytes:w+i length:k*4];i+=k;}
            DIAG_WORDS((4u<<16)|61,boolean,loaded,helper);
            DIAG_WORDS((3u<<16)|247,oldentry,0);
            DIAG_WORDS((4u<<16)|250,loaded,retblock,oldentry);
            DIAG_WORDS((2u<<16)|248,retblock);DIAG_WORDS((1u<<16)|253);
            DIAG_WORDS((2u<<16)|248,oldentry);injected=YES;continue;
        }
        if(inmain && injected && op==59) {
            nvlog("fragment helpers: variable outside relocated entry prefix; retaining original");
            return input;
        }
        [out appendBytes:w+i length:n*4];i+=n;
    }
#undef DIAG_WORDS
    if(!injected||!declared||!decorated){nvlog("fragment helpers: incomplete rewrite; retaining original");return input;}
    uint32_t fingerprint=2166136261u;
    for(size_t j=0;j<count;j++){fingerprint^=w[j];fingerprint*=16777619u;}
    nvlog("fragment helpers: invocation-independent physical-buffer module guarded (%zu words, bound %u, hash %08x)", count, w[3], fingerprint);return out;
}
