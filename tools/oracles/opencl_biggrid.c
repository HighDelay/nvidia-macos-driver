// OpenCL big-grid oracle: dispatches past 65535 work-groups on y, then on z, with a one-item work-group, and checks
// every item ran with the right global id and group id. Metal allows any group count per axis; Vulkan on NVK stops at
// 65535 on y and z (the QMD grid field width). Before 1.10 the driver passed the count through and NAK's nak_fill_qmd
// aborted the whole process (Geekbench 6 OpenCL, Face Detection, exit 134).
// Build: clang -O1 -framework OpenCL -Wno-deprecated-declarations opencl_biggrid.c -o opencl_biggrid
#include <OpenCL/opencl.h>
#include <stdio.h>
#include <stdlib.h>
static const char *src =
"__kernel void k(__global uint *o, uint axis) {\n"
"  uint i = get_global_id(axis);\n"
"  o[i] = (i + 1u) ^ ((uint)get_group_id(axis) << 1);\n"
"}\n";
enum { N = 70001 };
static int run(cl_context c, cl_command_queue q, cl_kernel k, cl_uint axis) {
  cl_int e; cl_uint *h = calloc(N, sizeof *h);
  cl_mem o = clCreateBuffer(c, CL_MEM_READ_WRITE | CL_MEM_COPY_HOST_PTR, N * sizeof *h, h, &e);
  clSetKernelArg(k, 0, sizeof o, &o); clSetKernelArg(k, 1, sizeof axis, &axis);
  size_t g[3] = { 1, 1, 1 }, l[3] = { 1, 1, 1 }; g[axis] = N;
  e = clEnqueueNDRangeKernel(q, k, 3, NULL, g, l, 0, NULL, NULL);
  clFinish(q); clEnqueueReadBuffer(q, o, CL_TRUE, 0, N * sizeof *h, h, 0, NULL, NULL);
  int bad = 0, first = -1;
  for (int i = 0; i < N; i++) if (h[i] != (((cl_uint)i + 1u) ^ ((cl_uint)i << 1))) { bad++; if (first < 0) first = i; }
  printf("axis %u: %d groups, enqueue %d, %s (%d wrong, first %d)\n", axis, N, e, bad ? "WRONG" : "OK", bad, first);
  clReleaseMemObject(o); free(h);
  return bad != 0 || e != 0;
}
int main(void) {
  cl_platform_id p; cl_device_id d; cl_int e;
  clGetPlatformIDs(1, &p, NULL); e = clGetDeviceIDs(p, CL_DEVICE_TYPE_GPU, 1, &d, NULL); if (e) { printf("no GPU %d\n", e); return 1; }
  char name[256]; clGetDeviceInfo(d, CL_DEVICE_NAME, sizeof name, name, NULL); printf("device %s\n", name);
  cl_context c = clCreateContext(NULL, 1, &d, NULL, NULL, &e);
  cl_command_queue q = clCreateCommandQueue(c, d, 0, &e);
  cl_program pr = clCreateProgramWithSource(c, 1, &src, NULL, &e);
  e = clBuildProgram(pr, 1, &d, NULL, NULL, NULL); if (e) { printf("build %d\n", e); return 1; }
  cl_kernel k = clCreateKernel(pr, "k", &e);
  int bad = run(c, q, k, 0) | run(c, q, k, 1) | run(c, q, k, 2);
  printf("result %s\n", bad ? "WRONG" : "OK");
  return bad;
}
