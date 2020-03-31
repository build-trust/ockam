# System Info
SET(CMAKE_SYSTEM_NAME "Linux")
set(CMAKE_SYSTEM_PROCESSOR "armv8")
SET(CMAKE_SYSTEM_VERSION 1)

set(OCKAM_TARGET_ARCHITECTURE "armv8")
set(OCKAM_TARGET_VENDOR "rpi3")
set(OCKAM_TARGET_OS "linux")
set(OCKAM_TARGET_LIBC "gnueabihf")
set(OCKAM_TARGET_TRIPLE
  "${OCKAM_TARGET_ARCHITECTURE}-${OCKAM_TARGET_VENDOR}-${OCKAM_TARGET_OS}-${OCKAM_TARGET_LIBC}")

# Define the cross compiler locations
set(CMAKE_C_COMPILER  ${OCKAM_C_COMPILER_PATH}/arm-linux-gnueabihf-gcc)
set(CMAKE_CXX_COMPILER ${OCKAM_C_COMPILER_PATH}/arm-linux-gnueabihf-g++)

# Where is the target environment
SET(CMAKE_FIND_ROOT_PATH ${OCKAM_C_SYSROOT_PATH}/arm-linux-gnueabihf/sysroot)

# Search for programs in the build host directories
SET(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)

# For libraries and headers in the target directories
SET(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
SET(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)

# Remove Apple
UNSET(APPLE)

if(UNIX AND NOT APPLE)
    set(LINUX TRUE)
endif()
