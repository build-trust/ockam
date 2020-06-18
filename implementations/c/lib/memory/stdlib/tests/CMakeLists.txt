project(ockam_stdlib_memory_tests C)
message(STATUS "----------------- Ockam Stdlib Memory Tests ----------------")
ockam_cc_test(
    NAME
        full
    SRCS
        stdlib_memory_test.c
    INCLUDES
        ${CMOCKA_INCLUDE_DIRS}
    DEPS
        ockam::memory::stdlib
        ockam::log
)
