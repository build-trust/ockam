set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/cmocka/cmocka/include)

set(UNIT_TESTING TRUE CACHE BOOL "")
set(CMAKE_EXPORT_COMPILE_COMMANDS OFF CACHE BOOL "")

add_subdirectory(${THIRD_PARTY_DIR}/cmocka/cmocka cmocka)
