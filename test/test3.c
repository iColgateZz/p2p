#define PSH_BUILD_IMPL
#define PSH_CORE_NO_PREFIX
#include "psh_build/psh_build.h"

i32 main(i32 argc, byte *argv[]) {
    PSH_REBUILD_UNITY(argc, argv);

    printf("Hello, world!\n");
}