cmake_minimum_required(VERSION 3.4 FATAL_ERROR)

project(c_scanners_ffi CXX)
set(CMAKE_CXX_STANDARD 20)

add_library(c_scanners_ffi STATIC ffi.cpp)
set_target_properties(c_scanners_ffi
    PROPERTIES
        ARCHIVE_OUTPUT_DIRECTORY ${PROJECT_SOURCE_DIR}/../lib/
)

set(CMAKE_CXX_FLAGS "-O3 -mavx2 -mbmi -mbmi2 -mssse3 -mlzcnt")