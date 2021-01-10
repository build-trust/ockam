cmake_minimum_required(VERSION 3.14)

project(example_memory
  VERSION 1.0
)

# Retrieve the Ockam C Library
get_filename_component(ockam_DIR "../../../" REALPATH BASE_DIR "${CMAKE_CURRENT_LIST_DIR}")

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
add_executable(example_memory "")

target_sources(example_memory
  PUBLIC
    example_memory.c
  )

target_link_libraries(example_memory
  ockam::memory
  ockam::memory::stdlib
)

target_include_directories(example_memory
  PUBLIC
    ${OCKAM_INCLUDES_OUTPUT_DIRECTORY}
  )
