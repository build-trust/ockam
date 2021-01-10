cmake_minimum_required(VERSION 3.14)

project(example_vault_initialize_default
  VERSION 1.0
)

# Retrieve the Ockam C Library
get_filename_component(ockam_DIR "../../../../../" REALPATH BASE_DIR "${CMAKE_CURRENT_LIST_DIR}")

include(FetchContent)
FetchContent_Declare(ockam
  URL "${ockam_DIR}"
)
FetchContent_Populate(ockam
  SOURCE_DIR "${ockam_DIR}"
)

# Include the Ockam C Library
add_subdirectory(${ockam_SOURCE_DIR} ${ockam_BINARY_DIR})

# Add the necessary libraries and sources for the example executable
add_executable(example_vault_initialize_default "")

target_sources(example_vault_initialize_default
  PUBLIC
    main.c
  )

target_link_libraries(example_vault_initialize_default
  ockam::memory
  ockam::memory::stdlib
  ockam::random
  ockam::random::urandom
  ockam::vault
  ockam::vault::default
)

target_include_directories(example_vault_initialize_default
  PUBLIC
    ${OCKAM_INCLUDES_OUTPUT_DIRECTORY}
  )
