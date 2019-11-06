cmake_minimum_required(VERSION 3.13)
project(ockam_mem)


###########################
# Set directory locations #
###########################

set(MEM_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../include)


#################
# Build Options #
#################

# Mem Build Options
option(MEM_LINUX "MEM: Linux")
option(MEM_FREERTOS "MEM: FreeRTOS")


####################
# Mem Library Code #
####################

if(MEM_STDLIB)
    add_definitions(-DMEM_STDLIB)
    set(MEM_SRC ${MEM_SRC_DIR}/ockam_mem_stdlib.c)
endif()


###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_mem ${MEM_SRC})

# Set the include directories
target_include_directories(ockam_mem PUBLIC ${OCKAM_INC_DIR})

# Set the target
set_property(TARGET ockam_mem PROPERTY C_STANDARD 99)

# Add any extra libs
target_link_libraries(ockam_mem)

