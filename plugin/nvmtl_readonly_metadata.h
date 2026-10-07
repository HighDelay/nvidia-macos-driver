/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

static NSData *nvmtl_preserve_readonly_metadata(NSData *input) {
    const uint32_t *w=input.bytes; size_t n=input.length/4;
    if(input.length%4||n<5||w[0]!=0x07230203||!w[3]||w[3]>4194304)return input;
    uint32_t bound=w[3],next=bound; size_t firsttype=0,firstfn=0;
    uint32_t *typeoff=calloc(bound,4),*readonly=calloc(bound,4),*replacement=calloc(bound,4);
    if(!typeoff||!readonly||!replacement){free(typeoff);free(readonly);free(replacement);return input;}
    BOOL valid=YES,logical=NO;
    for(size_t i=5;i<n;){uint32_t c=w[i]>>16,op=w[i]&65535;
        if(!c||i+c>n){valid=NO;break;}
        if(op==14&&c==3)logical=w[i+1]==0;
        if(op>=19&&op<=39&&c>=2&&w[i+1]<bound){typeoff[w[i+1]]=(uint32_t)i;if(!firsttype)firsttype=i;}
        if(op==54&&!firstfn)firstfn=i;
        if(op==71&&c==3&&w[i+1]<bound&&w[i+2]==24)readonly[w[i+1]]=1;
        i+=c;
    }
    NSMutableData *ann=[NSMutableData data];NSMutableDictionary<NSNumber*,NSData*> *insert=[NSMutableDictionary new];
    unsigned changed=0;
    if(valid&&logical&&firsttype&&firstfn)for(size_t i=firsttype;i<firstfn;){uint32_t c=w[i]>>16,op=w[i]&65535;
        if(op==59&&c>=4&&w[i+2]<bound&&readonly[w[i+2]]&&w[i+3]==12&&w[i+1]<bound){
            uint32_t root=w[i+2],po=typeoff[w[i+1]],st=0,so=0;
            if(po&&(w[po]&65535)==32&&(w[po]>>16)==4&&w[po+2]==12&&w[po+3]<bound){st=w[po+3];so=typeoff[st];}
            BOOL safe=so&&(w[so]&65535)==30;
            for(size_t j=5;safe&&j<firsttype;){uint32_t jc=w[j]>>16,jo=w[j]&65535;
                if(jo==74)for(uint32_t k=2;k<jc;k++)if(w[j+k]==st)safe=NO;
                if(jo==75)for(uint32_t k=2;k+1<jc;k+=2)if(w[j+k]==st)safe=NO;
                if((jo==332||jo==5632||jo==5633)&&jc>=3&&w[j+1]==st)safe=NO;
                j+=jc;
            }
            for(size_t j=firstfn;safe&&j<n;){uint32_t jc=w[j]>>16,jo=w[j]&65535;
                for(uint32_t k=1;k<jc;k++)if(w[j+k]==root&&!((jo==65||jo==66)&&k==3)){safe=NO;break;}
                j+=jc;
            }
            for(size_t j=firsttype;safe&&j<firstfn;){uint32_t jc=w[j]>>16,jo=w[j]&65535;
                if(jo==59&&j!=i&&jc>4&&w[j+4]==root)safe=NO;j+=jc;
            }
            if(safe){uint32_t ns=next++,np=next++,members=(w[so]>>16)-2;
                NSMutableData *decl=[NSMutableData dataWithBytes:w+so length:(w[so]>>16)*4];((uint32_t*)decl.mutableBytes)[1]=ns;
                uint32_t pt[]={ (4u<<16)|32,np,12,ns };[decl appendBytes:pt length:sizeof pt];insert[@(i)]=decl;replacement[root]=np;
                NSMutableIndexSet *marked=[NSMutableIndexSet indexSet];
                for(size_t j=5;j<firsttype;){uint32_t jc=w[j]>>16,jo=w[j]&65535;
                    if((jo==71||jo==72)&&jc>=3&&w[j+1]==st){NSMutableData *a=[NSMutableData dataWithBytes:w+j length:jc*4];((uint32_t*)a.mutableBytes)[1]=ns;[ann appendData:a];
                        if(jo==72&&jc==4&&w[j+3]==24)[marked addIndex:w[j+2]];}
                    j+=jc;
                }
                for(uint32_t member=0;member<members;member++)if(![marked containsIndex:member]){uint32_t a[]={ (4u<<16)|72,ns,member,24 };[ann appendBytes:a length:sizeof a];}
                changed++;
            }
        }i+=c;
    }
    NSMutableData *out=nil;
    if(changed){out=[NSMutableData dataWithBytes:w length:firsttype*4];[out appendData:ann];
        for(size_t i=firsttype;i<n;){uint32_t c=w[i]>>16,op=w[i]&65535;NSData *extra=insert[@(i)];if(extra)[out appendData:extra];
            if(op==59&&c>=4&&w[i+2]<bound&&replacement[w[i+2]]){NSMutableData *v=[NSMutableData dataWithBytes:w+i length:c*4];((uint32_t*)v.mutableBytes)[1]=replacement[w[i+2]];[out appendData:v];}
            else [out appendBytes:w+i length:c*4];i+=c;}
        ((uint32_t*)out.mutableBytes)[3]=next;nvlog("read-only metadata: preserved %u descriptor guarantees",changed);
    }
    free(typeoff);free(readonly);free(replacement);return out?:input;
}
