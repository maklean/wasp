#pragma once

#include "function_attributes.hpp"

#include <cstdio>
#include <cstdlib>

struct ReleaseAssertFailure
{
    static void NO_RETURN Fire(const char *__assertion, const char *__file,
	                 unsigned int __line, const char *__function)
	{
        fprintf(stderr, "%s:%u: %s: Assertion `%s' failed.\n", __file, __line, __function, __assertion);
		abort();
	}
};

#define ReleaseAssert(expr)							\
     (static_cast <bool> (expr)						\
      ? void (0)							        \
      : ReleaseAssertFailure::Fire(#expr, __FILE__, __LINE__, __extension__ __PRETTY_FUNCTION__))