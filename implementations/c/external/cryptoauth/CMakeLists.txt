if (NOT OCKAM_ENABLE_ATECC608A_BUILD)
    return()
endif()

find_package(cryptoauth QUIET)

message(STATUS "FIND")

if(NOT cryptoauth_FOUND)
    message(STATUS "NOT FOUND")

    FetchContent_Declare(
            cryptoauth
            GIT_REPOSITORY https://github.com/MicrochipTech/cryptoauthlib.git
            GIT_TAG v3.1.0
    )

    set(ATCA_HAL_I2C ON CACHE BOOL "")
    set(ATCA_BUILD_SHARED_LIBS OFF CACHE BOOL "")

    FetchContent_GetProperties(cryptoauth)
    if(NOT cryptoauth_POPULATED)
        FetchContent_Populate(cryptoauth)
        add_subdirectory(${cryptoauth_SOURCE_DIR} ${cryptoauth_BINARY_DIR} EXCLUDE_FROM_ALL)
    endif()

    # https://gitlab.kitware.com/cmake/cmake/-/issues/17735
    file(WRITE ${CMAKE_BINARY_DIR}/find/cryptoauth/cryptoauthConfig.cmake "")
    set(cryptoauth_DIR ${CMAKE_BINARY_DIR}/find/cryptoauth CACHE PATH "" FORCE)
endif()