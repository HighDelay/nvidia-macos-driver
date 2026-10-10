// OpenCL integer-image oracle: a convolution that accumulates in float4 and stores with write_imageui(convert_uint4_sat()),
// the shape of Geekbench 6's Face Detection convolve_{horizontal,vertical}_img. Before 1.10 the translator typed the image
// as float and refused the uint4 texel ("air.write_texture: unsupported texel shape"), so the kernel never built.
// Build: clang -O1 -framework OpenCL -Wno-deprecated-declarations opencl_imageui.c -o opencl_imageui
#include <OpenCL/opencl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
static const char *src =
"__constant sampler_t s = CLK_NORMALIZED_COORDS_FALSE | CLK_ADDRESS_CLAMP_TO_EDGE | CLK_FILTER_NEAREST;\n"
"__kernel void conv(__read_only image2d_t in, __write_only image2d_t out, __constant float *w, int taps) {\n"
"  int2 p = (int2)(get_global_id(0), get_global_id(1)); float4 sum = 0.0f;\n"
"  for (int i = 0; i < taps; i++) sum += convert_float4(read_imageui(in, s, p + (int2)(i - taps / 2, 0))) * w[i];\n"
"  write_imageui(out, p, convert_uint4_sat(sum));\n"
"}\n";
enum { W = 64, H = 16, TAPS = 3 };
int main(void) {
  cl_platform_id p; cl_device_id d; cl_int e;
  clGetPlatformIDs(1, &p, NULL); e = clGetDeviceIDs(p, CL_DEVICE_TYPE_GPU, 1, &d, NULL); if (e) { printf("no GPU %d\n", e); return 1; }
  cl_context c = clCreateContext(NULL, 1, &d, NULL, NULL, &e);
  cl_command_queue q = clCreateCommandQueue(c, d, 0, &e);
  // ORACLE_PAUSE=<s>: wait once the driver has loaded (device, context, queue), before the program and kernel exist,
  // so dtrace can attach to the plugin's methods
  if (getenv("ORACLE_PAUSE")) { printf("pid %d paused\n", getpid()); fflush(stdout); sleep((unsigned)atoi(getenv("ORACLE_PAUSE"))); }
  cl_program pr = clCreateProgramWithSource(c, 1, &src, NULL, &e);
  e = clBuildProgram(pr, 1, &d, NULL, NULL, NULL); printf("build %d\n", e); if (e) return 1;
  cl_kernel k = clCreateKernel(pr, "conv", &e); printf("kernel %d\n", e); if (e) return 1;
  cl_image_format f = { CL_RGBA, CL_UNSIGNED_INT8 };
  cl_image_desc dd = { CL_MEM_OBJECT_IMAGE2D, W, H };
  unsigned char hin[W * H * 4], hout[W * H * 4] = { 0 };
  for (int i = 0; i < W * H * 4; i++) hin[i] = (unsigned char)(i * 7);
  cl_mem in = clCreateImage(c, CL_MEM_READ_ONLY | CL_MEM_COPY_HOST_PTR, &f, &dd, hin, &e);
  cl_mem out = clCreateImage(c, CL_MEM_WRITE_ONLY, &f, &dd, NULL, &e);
  float hw[TAPS] = { 0.25f, 0.5f, 0.25f }; int taps = TAPS;
  cl_mem w = clCreateBuffer(c, CL_MEM_READ_ONLY | CL_MEM_COPY_HOST_PTR, sizeof hw, hw, &e);
  cl_uint na = 0; clGetKernelInfo(k, CL_KERNEL_NUM_ARGS, sizeof na, &na, NULL);
  printf("num_args %u, image in %d out %d\n", na, in != NULL, out != NULL);
  printf("arg0 image %d\n", clSetKernelArg(k, 0, sizeof in, &in));
  printf("arg1 image %d\n", clSetKernelArg(k, 1, sizeof out, &out));
  printf("arg2 constant %d\n", clSetKernelArg(k, 2, sizeof w, &w));
  printf("arg3 int %d\n", clSetKernelArg(k, 3, sizeof taps, &taps));
  size_t g[2] = { W, H };
  printf("enqueue %d\n", clEnqueueNDRangeKernel(q, k, 2, NULL, g, NULL, 0, NULL, NULL));
  clFinish(q);
  size_t o[3] = { 0 }, r[3] = { W, H, 1 };
  clEnqueueReadImage(q, out, CL_TRUE, o, r, 0, 0, hout, 0, NULL, NULL);
  int bad = 0, first = -1;
  for (int y = 0; y < H; y++) for (int x = 0; x < W; x++) for (int ch = 0; ch < 4; ch++) {
    float sum = 0;
    for (int i = 0; i < TAPS; i++) { int xx = x + i - TAPS / 2; xx = xx < 0 ? 0 : xx >= W ? W - 1 : xx; sum += hin[(y * W + xx) * 4 + ch] * hw[i]; }
    int want = (int)(sum + 0.5f) > 255 ? 255 : (int)(sum + 0.5f), got = hout[(y * W + x) * 4 + ch];
    if (got != want && got != (int)sum) { bad++; if (first < 0) first = (y * W + x) * 4 + ch; }
  }
  printf("result %s (%d wrong of %d, first %d)\n", bad ? "WRONG" : "OK", bad, W * H * 4, first);
  return bad != 0;
}
