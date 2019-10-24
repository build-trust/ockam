SET(CMAKE_SYSTEM_NAME Linux)
SET(CMAKE_SYSTEM_PROCESSOR arm)

SET(CMAKE_C_COMPILER /home/vagrant/x-tools/arm-unknown-linux-gnueabi/bin/arm-unknown-linux-gnueabi-gcc)
SET(CMAKE_CXX_COMPILER /home/vagrant/x-tools/arm-unknown-linux-gnueabi/bin/arm-unknown-linux-gnueabi-g++)

SET(CMAKE_FIND_ROOT_PATH /home/vagrant/x-tools/arm-unknown-linux-gnueabi/)

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)
