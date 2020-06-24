project(ockam_memory_tests C)
message(STATUS "----------------- Ockam Memory Tests ----------------")
ockam_cc_test(
    NAME
        full
    SRCS
        memory_compare_test.c
    INCLUDES
        ${CMOCKA_INCLUDE_DIRS}
    DEPS
        ockam::memory::stdlib
        ockam::log
)
