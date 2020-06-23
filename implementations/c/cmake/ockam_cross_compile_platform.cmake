# Supported target triples (case-sensitive):
#   Linux: x86_64-unknown-linux     "-DOCKAM_TARGET_PLATFORM=linux"
#   Mac: x86_64-apple-darwin        "-DOCKAM_TARGET_PLATFORM=mac"
#   Raspberry Pi: armv8-rpi3-linux  "-DOCKAM_TARGET_PLATFORM=pi"

# Set up target architecture
if(DEFINED OCKAM_TARGET_PLATFORM)
  if(${OCKAM_TARGET_PLATFORM} STREQUAL "pi")
    include(${CMAKE_MODULE_PATH}/platform/raspberry-pi.cmake)
  elseif(${OCKAM_TARGET_PLATFORM} STREQUAL "pi-mac")
    include(${CMAKE_MODULE_PATH}/platform/raspberry-pi-mac.cmake)
  elseif(${OCKAM_TARGET_PLATFORM} STREQUAL "mac")
    include(${CMAKE_MODULE_PATH}/platform/mac.cmake)
  elseif(${OCKAM_TARGET_PLATFORM} STREQUAL "linux")
    include(${CMAKE_MODULE_PATH}/platform/linux.cmake)
  else()
    message(FATAL_ERROR "OCKAM_TARGET_PLATFORM was specified, but is not valid. Expected one of [pi, pi-mac, mac, linux]")
  endif()
elseif(DEFINED OCKAM_TARGET_TRIPLE)
  if("${OCKAM_TARGET_TRIPLE}" MATCHES "x86_64-apple-darwin")
    include(${CMAKE_MODULE_PATH}/platform/mac.cmake)
  elseif("${OCKAM_TARGET_TRIPLE}" MATCHES "x86_64-unknown-linux")
    include(${CMAKE_MODULE_PATH}/platform/linux.cmake)
  elseif("${OCKAM_TARGET_TRIPLE}" MATCHES "armv8-rpi3-linux")
    include(${CMAKE_MODULE_PATH}/platform/raspberry-pi.cmake)
  else()
    message(FATAL_ERROR "OCKAM_TARGET_TRIPLE was specified, but the specified target is not recognized.")
  endif()
endif()
