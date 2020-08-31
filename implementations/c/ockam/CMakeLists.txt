add_subdirectory(error)

add_subdirectory(log)

add_subdirectory(memory)
add_subdirectory(memory/stdlib)

add_subdirectory(random)

if (NOT WIN32)
    add_subdirectory(random/urandom)
endif()

add_subdirectory(io)

add_subdirectory(queue)

add_subdirectory(mutex)
add_subdirectory(mutex/pthread)

add_subdirectory(vault)
add_subdirectory(vault/default)
# add_subdirectory(vault/rust)

if (OCKAM_ENABLE_ATECC608A_BUILD)
    add_subdirectory(vault/atecc608a)
    # add_subdirectory(vault/atecc508a)
endif()

add_subdirectory(transport)

if (NOT WIN32)
    add_subdirectory(transport/posix_socket)
endif()

add_subdirectory(key_agreement)
add_subdirectory(key_agreement/xx)

add_subdirectory(codec)

add_subdirectory(channel)

# add_subdirectory(router)
