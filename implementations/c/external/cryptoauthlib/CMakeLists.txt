if (NOT OCKAM_ENABLE_ATECC608A_BUILD)
    return()
endif()

find_package(cryptoauthlib QUIET)

if(NOT cryptoauthlib_FOUND)
    FetchContent_Declare(
            cryptoauthlib
            GIT_REPOSITORY https://github.com/MicrochipTech/cryptoauthlib.git
            GIT_TAG v3.1.0
    )

    set(ATCA_HAL_I2C ON CACHE BOOL "")
    set(ATCA_BUILD_SHARED_LIBS OFF CACHE BOOL "")

    FetchContent_GetProperties(cryptoauthlib)
    if(NOT cryptoauthlib_POPULATED)
        FetchContent_Populate(cryptoauthlib)
        add_subdirectory(${cryptoauthlib_SOURCE_DIR} ${cryptoauthlib_BINARY_DIR} EXCLUDE_FROM_ALL)
    endif()

    # https://gitlab.kitware.com/cmake/cmake/-/issues/17735
    file(WRITE ${CMAKE_BINARY_DIR}/find/cryptoauthlib/cryptoauthlibConfig.cmake "")
    set(cryptoauthlib_DIR ${CMAKE_BINARY_DIR}/find/cryptoauthlib CACHE PATH "" FORCE)
endif()