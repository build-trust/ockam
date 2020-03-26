# Define functions to get the host and target triple.
#
# The triple has the general format <arch><sub>-<vendor>-<sys>-<abi>, where:
# arch = x86_64, i386, arm, thumb, mips, etc.
# sub = for ex. on ARM: v5, v6m, v7a, v7m, etc.
# vendor = pc, apple, nvidia, ibm, etc.
# sys = none, linux, win32, darwin, cuda, etc.
# abi = eabi, gnu, android, macho, elf, etc.

function(get_host_triple out out_arch out_vendor out_os)

  # Get the architecture.
  cmake_host_system_information(RESULT _os_platform QUERY OS_PLATFORM)

  if(_os_platform STREQUAL "x86_64")
   set(arch "x86_64")
  elseif(_os_platform STREQUAL "i686")
   # set(arch "i686")
  else()
   set(arch "unknown")
  endif()

  # Get the vendor.
  cmake_host_system_information(RESULT _os_host_system QUERY OS_NAME)

  if (${_os_host_system} STREQUAL "Mac OS X")
   set(vendor "apple")
   set(_os_host_system "Darwin")
  else()
   set(vendor "pc")
  endif()

  # Get Os.
  if (${_os_host_system} STREQUAL "Windows")
   set(os "win32")
  else()
   string(TOLOWER ${_os_host_system} os)
  endif()

  set(triple "${arch}-${vendor}-${os}")
  set(${out} ${triple} PARENT_SCOPE)
  set(${out_arch} ${arch} PARENT_SCOPE)
  set(${out_vendor} ${vendor} PARENT_SCOPE)
  set(${out_os} ${os} PARENT_SCOPE)

endfunction()


function(get_target_triple out out_arch out_vendor out_os)

  # Get the architecture.
  set(arch ${OCKAM_TARGET_ARCHITECTURE})

  # Get the vendor.
  if(${OCKAM_TARGET_OS} STREQUAL "Darwin")
    set(vendor "apple")
  else()
    set(vendor ${OCKAM_TARGET_VENDOR})
  endif()

  # Get OS.
  if (${OCKAM_TARGET_OS} STREQUAL "Windows")
   set(os "win32")
  else()
   string(TOLOWER ${OCKAM_TARGET_OS} os)
  endif()

  set(triple "${arch}-${vendor}-${os}")
  set(${out} ${triple} PARENT_SCOPE)
  set(${out_arch} ${arch} PARENT_SCOPE)
  set(${out_vendor} ${vendor} PARENT_SCOPE)
  set(${out_os} ${os} PARENT_SCOPE)

endfunction()
