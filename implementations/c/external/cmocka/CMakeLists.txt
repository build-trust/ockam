
if(BUILD_TESTING AND NOT (CMAKE_VERSION VERSION_LESS 3.5))

  find_package(cmocka 1.1.5 QUIET)

  if(NOT cmocka_FOUND)
    FetchContent_Declare(
      cmocka
      GIT_REPOSITORY https://git.cryptomilk.org/projects/cmocka.git
      GIT_TAG cmocka-1.1.5
    )

    set(WITH_STATIC_LIB ON CACHE BOOL "")
    set(WITH_EXAMPLES OFF CACHE BOOL "")

    FetchContent_GetProperties(cmocka)
    if(NOT cmocka_POPULATED)
      FetchContent_Populate(cmocka)
      add_subdirectory(${cmocka_SOURCE_DIR} ${cmocka_BINARY_DIR} EXCLUDE_FROM_ALL)
    endif()

    # https://gitlab.kitware.com/cmake/cmake/-/issues/17735
    file(WRITE ${CMAKE_BINARY_DIR}/find/cmocka/cmockaConfig.cmake "")
    set(cmocka_DIR ${CMAKE_BINARY_DIR}/find/cmocka CACHE PATH "" FORCE)
  endif()

endif()
