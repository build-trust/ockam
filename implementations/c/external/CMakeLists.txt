
# add all subdirectories

file(GLOB children RELATIVE ${CMAKE_CURRENT_SOURCE_DIR} ${CMAKE_CURRENT_SOURCE_DIR}/*)

foreach(child ${children})
  if(IS_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/${child})
    add_subdirectory(${child})
  endif()
endforeach()
