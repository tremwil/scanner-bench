cmake_minimum_required(VERSION 3.4 FATAL_ERROR)

project(c_scanners_ffi CXX)
set(CMAKE_CXX_STANDARD 20)

add_library(c_scanners_ffi SHARED ffi.cpp)
set_target_properties(c_scanners_ffi
    PROPERTIES
        LIBRARY_OUTPUT_DIRECTORY ${PROJECT_SOURCE_DIR}/../target/release/
        RUNTIME_OUTPUT_DIRECTORY ${PROJECT_SOURCE_DIR}/../target/release/
)

set(CMAKE_CXX_FLAGS "-O3 -mavx2 -mbmi -mbmi2 -mssse3 -mlzcnt")