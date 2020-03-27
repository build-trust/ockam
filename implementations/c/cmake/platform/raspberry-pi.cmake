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

# Define the RPI path
set(OCKAM_C_RPI_BASE ${OCKAM_C_BASE}/tools/toolchains/raspberrypi)
set(OCKAM_C_RPI_PATH ${OCKAM_C_BASE}/tools/toolchains/raspberrypi/tools/arm-bcm2708/arm-rpi-4.9.3-linux-gnueabihf)

# Define the cross compiler locations
SET(CMAKE_C_COMPILER  ${OCKAM_C_RPI_PATH}/bin/arm-linux-gnueabihf-gcc)
SET(CMAKE_CXX_COMPILER ${OCKAM_C_RPI_PATH}/bin/arm-linux-gnueabihf-g++)

# Where is the target environment
SET(CMAKE_FIND_ROOT_PATH ${OCKAM_C_RPI_PATH}/arm-linux-gnueabihf/sysroot)

# Search for programs in the build host directories
SET(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)

# For libraries and headers in the target directories
SET(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
SET(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)

# Remove Apple
UNSET(APPLE)

if(UNIX)
    message(STATUS ">> Unix")
endif()

if(APPLE)
    message(STATUS ">> Apple")
endif()

if(UNIX AND NOT APPLE)
    set(LINUX TRUE)
endif()

# if(NOT LINUX) should work, too, if you need that
if(LINUX)
    message(STATUS ">>> Linux")
    # linux stuff here
else()
    message(STATUS ">>> Not Linux")
    # stuff that should happen not on Linux
endif()



#if(NOT EXISTS ${OCKAM_C_RPI_PATH})
#  message(STATUS "Cloning pi tools into '${OCKAM_C_RPI_BASE}'")
#  #!! get rid of literal URL
#  execute_process(COMMAND git clone https://github.com/raspberrypi/tools.git
#    WORKING_DIRECTORY ${OCKAM_C_RPI_BASE}
#    RESULT_VARIABLE _RESULT)
#  execute_process(COMMAND git checkout 4a335520900ce55e251ac4f420f52bf0b2ab6b1f
#    WORKING_DIRECTORY ${OCKAM_C_RPI_BASE}/tools
#    RESULT_VARIABLE _RESULT)
#endif()
