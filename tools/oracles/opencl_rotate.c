// OpenCL rotate oracle: a bilinear image rotation over a 5600x4200 4-channel byte image with a per-channel loop, the
// shape and size of Geekbench 6's Horizon Detection image_rotate. On the 1.10 candidate that workload never completed
// (the GPU submission did not retire, no fault logged); this checks whether the kernel itself is the cause.
// Build: clang -O1 -framework OpenCL -Wno-deprecated-declarations opencl_rotate.c -o opencl_rotate
#include <OpenCL/opencl.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
static const char *src =
"__kernel void image_rotate(__global const uchar *src, __global uchar *dst, int src_width, int src_height,\n"
"                           int src_stride, int dst_width, int dst_height, int channels, float angle) {\n"
"  int x = get_global_id(0), y = get_global_id(1);\n"
"  if (x >= dst_width || y >= dst_height) return;\n"
"  float a = angle * -0.017453292f, cs = cos(a), sn = sin(a);\n"
"  float cx = (float)(x - dst_width / 2), cy = (float)(y - dst_height / 2);\n"
"  float fx = cs * cx - sn * cy, fy = sn * cx + cs * cy;\n"
"  float x0 = floor(fx), y0 = floor(fy), dx = fx - x0, dy = fy - y0;\n"
"  int ix0 = clamp((int)x0 + src_width / 2, 0, src_width - 1), ix1 = clamp((int)ceil(fx) + src_width / 2, 0, src_width - 1);\n"
"  int iy0 = clamp((int)y0 + src_height / 2, 0, src_height - 1), iy1 = clamp((int)ceil(fy) + src_height / 2, 0, src_height - 1);\n"
"  for (int c = 0; c < channels; c++) {\n"
"    float p00 = src[(iy0 * src_stride + ix0) * channels + c], p01 = src[(iy0 * src_stride + ix1) * channels + c];\n"
"    float p10 = src[(iy1 * src_stride + ix0) * channels + c], p11 = src[(iy1 * src_stride + ix1) * channels + c];\n"
"    float top = p00 + (p01 - p00) * dx, bot = p10 + (p11 - p10) * dx;\n"
"    dst[(y * dst_width + x) * channels + c] = convert_uchar(clamp(top + (bot - top) * dy, 0.0f, 255.0f));\n"
"  }\n"
"}\n";
enum { SW = 5600, SH = 4200, STRIDE = 5632, DW = 5600, DH = 4200, CH = 4 };
int main(void) {
  cl_platform_id p; cl_device_id d; cl_int e;
  clGetPlatformIDs(1, &p, NULL); e = clGetDeviceIDs(p, CL_DEVICE_TYPE_GPU, 1, &d, NULL); if (e) { printf("no GPU %d\n", e); return 1; }
  cl_context c = clCreateContext(NULL, 1, &d, NULL, NULL, &e);
  cl_command_queue q = clCreateCommandQueue(c, d, 0, &e);
  // ORACLE_PAUSE=<s>: wait once the driver is loaded, before any program or buffer exists, so dtrace can attach
  if (getenv("ORACLE_PAUSE")) { printf("pid %d paused\n", getpid()); fflush(stdout); sleep((unsigned)atoi(getenv("ORACLE_PAUSE"))); }
  cl_program pr = clCreateProgramWithSource(c, 1, &src, NULL, &e);
  e = clBuildProgram(pr, 1, &d, "-cl-fast-relaxed-math", NULL, NULL); printf("build %d\n", e); if (e) return 1;
  cl_kernel k = clCreateKernel(pr, "image_rotate", &e); printf("kernel %d\n", e); if (e) return 1;
  size_t sn = (size_t)STRIDE * SH * CH, dn = (size_t)DW * DH * CH;
  unsigned char *hs = malloc(sn), *hd = calloc(dn, 1);
  for (size_t i = 0; i < sn; i++) hs[i] = (unsigned char)((i * 2654435761u) >> 24);
  // ROT_SRC=write: a driver-allocated source filled by clEnqueueWriteBuffer instead of CL_MEM_COPY_HOST_PTR
  int src_write = getenv("ROT_SRC") && !strcmp(getenv("ROT_SRC"), "write");
  cl_mem bs = src_write ? clCreateBuffer(c, CL_MEM_READ_ONLY, sn, NULL, &e)
                        : clCreateBuffer(c, CL_MEM_READ_ONLY | CL_MEM_COPY_HOST_PTR, sn, hs, &e);
  if (src_write) clEnqueueWriteBuffer(q, bs, CL_TRUE, 0, sn, hs, 0, NULL, NULL);
  cl_mem bd = clCreateBuffer(c, CL_MEM_WRITE_ONLY, dn, NULL, &e);
  int sw = SW, sh = SH, st = STRIDE, dw = DW, dh = DH, ch = CH; float angle = 5.0f;
  clSetKernelArg(k, 0, sizeof bs, &bs); clSetKernelArg(k, 1, sizeof bd, &bd);
  clSetKernelArg(k, 2, sizeof sw, &sw); clSetKernelArg(k, 3, sizeof sh, &sh); clSetKernelArg(k, 4, sizeof st, &st);
  clSetKernelArg(k, 5, sizeof dw, &dw); clSetKernelArg(k, 6, sizeof dh, &dh); clSetKernelArg(k, 7, sizeof ch, &ch);
  clSetKernelArg(k, 8, sizeof angle, &angle);
  size_t g[2] = { DW, DH }, l[2] = { 32, 1 };
  // ROT_N=<n> dispatches (default 10), each timed to its clFinish
  int n = getenv("ROT_N") ? atoi(getenv("ROT_N")) : 10;
  for (int r = 0; r < n; r++) {
    struct timespec t0, t1; clock_gettime(CLOCK_MONOTONIC, &t0);
    clEnqueueNDRangeKernel(q, k, 2, NULL, g, l, 0, NULL, NULL); int f = clFinish(q);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    printf("dispatch %d (%s source): %.1f ms, finish %d\n", r, src_write ? "written" : "host-copied",
           (t1.tv_sec - t0.tv_sec) * 1e3 + (t1.tv_nsec - t0.tv_nsec) / 1e6, f); fflush(stdout);
  }
  clEnqueueReadBuffer(q, bd, CL_TRUE, 0, dn, hd, 0, NULL, NULL);
  // CPU reference on a sample of pixels; fast-math cos/sin may differ by a rounding step, so allow 1 and count >1
  long bad = 0, checked = 0;
  float a = angle * -0.017453292f, cs = cosf(a), snn = sinf(a);
  for (int y = 0; y < DH; y += 37) for (int x = 0; x < DW; x += 41) {
    float cx = (float)(x - DW / 2), cy = (float)(y - DH / 2), fx = cs * cx - snn * cy, fy = snn * cx + cs * cy;
    float x0 = floorf(fx), y0 = floorf(fy), dx = fx - x0, dy = fy - y0;
    int ix0 = (int)x0 + SW / 2, ix1 = (int)ceilf(fx) + SW / 2, iy0 = (int)y0 + SH / 2, iy1 = (int)ceilf(fy) + SH / 2;
    ix0 = ix0 < 0 ? 0 : ix0 > SW - 1 ? SW - 1 : ix0; ix1 = ix1 < 0 ? 0 : ix1 > SW - 1 ? SW - 1 : ix1;
    iy0 = iy0 < 0 ? 0 : iy0 > SH - 1 ? SH - 1 : iy0; iy1 = iy1 < 0 ? 0 : iy1 > SH - 1 ? SH - 1 : iy1;
    for (int c2 = 0; c2 < CH; c2++) {
      float p00 = hs[((size_t)iy0 * STRIDE + ix0) * CH + c2], p01 = hs[((size_t)iy0 * STRIDE + ix1) * CH + c2];
      float p10 = hs[((size_t)iy1 * STRIDE + ix0) * CH + c2], p11 = hs[((size_t)iy1 * STRIDE + ix1) * CH + c2];
      float top = p00 + (p01 - p00) * dx, bot = p10 + (p11 - p10) * dx, v = top + (bot - top) * dy;
      int want = v < 0 ? 0 : v > 255 ? 255 : (int)v, got = hd[((size_t)y * DW + x) * CH + c2];
      checked++; if (abs(got - want) > 1) bad++;
    }
  }
  printf("result %s (%ld of %ld sampled bytes off by more than 1)\n", bad ? "WRONG" : "OK", bad, checked);
  return bad != 0;
}
