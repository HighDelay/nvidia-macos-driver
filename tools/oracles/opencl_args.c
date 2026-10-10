// OpenCL argument oracle: one kernel with a global buffer, a __local buffer, a by-value int and an unused buffer.
// Every clSetKernelArg must return 0 and the result must be right. Before 1.10 OpenCL saw 0 arguments on this driver,
// every clSetKernelArg returned -49 (CL_INVALID_ARG_INDEX), and Geekbench's OpenCL run stopped at its first workload.
// Build: clang -O1 -framework OpenCL -Wno-deprecated-declarations opencl_args.c -o opencl_args
#include <OpenCL/opencl.h>
#include <stdio.h>
static const char *src =
"__kernel void k(__global float *a, __local float *tmp, int n, __global float *unused) {\n"
"  int i = get_global_id(0); int l = get_local_id(0);\n"
"  tmp[l] = a[i]; barrier(CLK_LOCAL_MEM_FENCE); a[i] = tmp[l] * n + get_local_size(0);\n"
"}\n";
int main(void) {
  cl_platform_id p; cl_device_id d; cl_int e;
  clGetPlatformIDs(1, &p, NULL); e = clGetDeviceIDs(p, CL_DEVICE_TYPE_GPU, 1, &d, NULL); if (e) { printf("no GPU %d\n", e); return 1; }
  char name[256]; clGetDeviceInfo(d, CL_DEVICE_NAME, sizeof name, name, NULL); printf("device %s\n", name);
  cl_context c = clCreateContext(NULL, 1, &d, NULL, NULL, &e);
  cl_command_queue q = clCreateCommandQueue(c, d, 0, &e);
  cl_program pr = clCreateProgramWithSource(c, 1, &src, NULL, &e);
  e = clBuildProgram(pr, 1, &d, NULL, NULL, NULL); printf("build %d\n", e);
  cl_kernel k = clCreateKernel(pr, "k", &e); printf("kernel %d\n", e);
  cl_uint na = 0; clGetKernelInfo(k, CL_KERNEL_NUM_ARGS, sizeof na, &na, NULL); printf("num_args %u\n", na);
  float h[64]; for (int i = 0; i < 64; i++) h[i] = i;
  cl_mem a = clCreateBuffer(c, CL_MEM_READ_WRITE | CL_MEM_COPY_HOST_PTR, sizeof h, h, &e);
  cl_mem u = clCreateBuffer(c, CL_MEM_READ_WRITE, 64, NULL, &e);
  int n = 2;
  printf("arg0 global %d\n", clSetKernelArg(k, 0, sizeof a, &a));
  printf("arg1 local %d\n", clSetKernelArg(k, 1, 16 * sizeof(float), NULL));
  printf("arg2 int %d\n", clSetKernelArg(k, 2, sizeof n, &n));
  printf("arg3 unused %d\n", clSetKernelArg(k, 3, sizeof u, &u));
  size_t g = 64, l = 16;
  printf("enqueue %d\n", clEnqueueNDRangeKernel(q, k, 1, NULL, &g, &l, 0, NULL, NULL));
  clFinish(q); clEnqueueReadBuffer(q, a, CL_TRUE, 0, sizeof h, h, 0, NULL, NULL);
  int bad = 0; for (int i = 0; i < 64; i++) if (h[i] != i * 2 + 16) bad++;
  printf("result %s (%d wrong; h[5]=%g want 26)\n", bad ? "WRONG" : "OK", bad, h[5]);
  return bad != 0;
}
