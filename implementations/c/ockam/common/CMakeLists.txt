cmake_minimum_required(VERSION 3.13)
project(ockam_common)


###########################
# Set directory locations #
###########################

set(COMMON_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(COMMON_SRC_DIR ${COMMON_DIR}/src)
set(OCKAM_DIR ${CMAKE_CURRENT_SOURCE_DIR}/..)

message( ${CMAKE_CURRENT_SOURCE_DIR} )


#################
# Build Options #
#################

set(CMAKE_VERBOSE_MAKEFILE TRUE)

# KAL Build Options
option(KAL_LINUX "KAL: Linux" ON)
option(KAL_FREERTOS "KAL: FreeRTOS" OFF)

# Log Build Options
option(LOG_LINUX "LOG: Linux" ON)
option(LOG_FREERTOS "LOG: FreeRTOS" OFF)

# Mem Build Options
option(MEM_STDLIB "MEM: Standard Library" ON)
option(MEM_CHUNK "MEM: Chunk" OFF)


#################
# Build Defines #
#################

# Set the base include directories
set(COMMON_INC ${OCKAM_DIR}/)

# Clear the src val
set(COMMON_SRC)
set(COMMON_LIB)


####################
# KAL Library Code #
####################

if(KAL_LINUX)
add_definitions(-DKAL_LINUX)
set(COMMON_SRC ${COMMON_SRC} ${COMMON_SRC_DIR}/kal/ockam_kal_linux.c)
endif()

if(KAL_FREERTOS)
add_definitions(-DKAL_FREERTOS)
set(COMMON_SRC ${COMMON_SRC} ${COMMON_SRC_DIR}/kal/ockam_kal_freertos.c)
endif()


####################
# Log Library Code #
####################

if(LOG_LINUX)
add_definitions(-DLOG_LINUX)
set(COMMON_SRC ${COMMON_SRC} ${COMMON_SRC_DIR}/log/ockam_log_linux.c)
endif()

if(LOG_FREERTOS)
add_definitions(-DLOG_FREERTOS)
set(COMMON_SRC ${COMMON_SRC} ${COMMON_SRC_DIR}/log/ockam_log_freertos.c)
endif()


####################
# Mem Library Code #
####################

if(MEM_STDLIB)
add_definitions(-DMEM_STDLIB)
set(COMMON_SRC ${COMMON_SRC} ${COMMON_SRC_DIR}/mem/ockam_mem_stdlib.c)
endif()

if(MEM_CHUNK)
add_definitions(-DMEM_CHUNK)
set(COMMON_SRC ${COMMON_SRC} ${COMMON_SRC_DIR}/mem/ockam_mem_chunk.c)
endif()


###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_common ${COMMON_SRC})

# Set the include directories
#target_include_directories(ockam_common PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}> PRIVATE ${COMMON_INC})
target_include_directories(ockam_common PUBLIC ${COMMON_INC})

# Set the target
set_property(TARGET ockam_common PROPERTY C_STANDARD 99)

# Add any extra libs
target_link_libraries(ockam_common)

