cmake_minimum_required(VERSION 3.14)

file(STRINGS "VERSION" OCKAM_VERSION)

project(ockam LANGUAGES C VERSION ${OCKAM_VERSION})

add_subdirectory("src/ockam/vault/ATECC")
install(TARGETS ockam_atecc_vault DESTINATION lib)

install(DIRECTORY include/ DESTINATION include/
        FILES_MATCHING PATTERN "*.h")
