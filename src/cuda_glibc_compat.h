/* CUDA 13.1 + glibc 2.42 compatibility shim.
 *
 * In C++11+ mode glibc's <sys/cdefs.h> expands __THROW to `noexcept(true)`.
 * CUDA 13.1's <crt/math_functions.h> declares rsqrt/rsqrtf without any
 * exception specifier, so when both headers are visible in the same
 * translation unit the compiler errors with:
 *   "exception specification is incompatible with that of previous function"
 *
 * Force __THROW to use only the nothrow attribute, matching the C-mode
 * definition and CUDA's declarations. */
#include <sys/cdefs.h>
/* Make __THROW/__NTH expand to nothing — same as cdefs.h's pre-C++11
 * fallback (lines 93-97). This avoids the noexcept(true) form that
 * conflicts with CUDA's earlier declarations of rsqrt/rsqrtf. */
#undef __THROW
#define __THROW
#undef __THROWNL
#define __THROWNL
#undef __NTH
#define __NTH(fct) fct
#undef __NTHNL
#define __NTHNL(fct) fct
