# Copy test fixtures to the right plact
set(_fixtures_dest_dir ${OCKAM_FIXTURES_OUTPUT_DIRECTORY})
set(_fixtures_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/fixtures)

file(GLOB _src_fixtures LIST_DIRECTORIES false "${_fixtures_src_dir}/*.txt")
file(MAKE_DIRECTORY ${_fixtures_dest_dir})
foreach(_src ${_src_fixtures})
  string(FIND ${_src} "/" _idx REVERSE)
  message("IDX '${_idx}'")
  math(EXPR _idx1 "${_idx} + 1")
  string(SUBSTRING ${_src} ${_idx1} -1 _filename)
  configure_file(${_src} ${_fixtures_dest_dir}/${_filename} )
endforeach()

add_subdirectory(posix_socket)
