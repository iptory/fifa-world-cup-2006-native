cmake_minimum_required(VERSION 3.25)
project(fifa_world_cup_2006 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(generated/rexglue.cmake)

set(FIFAWORLDCUP2006_SOURCES
    src/main.cpp
)

if(NOT WIN32)
    message(FATAL_ERROR "This project supports Windows only")
endif()
add_executable(fifa_world_cup_2006 WIN32 ${FIFAWORLDCUP2006_SOURCES})

rexglue_setup_target(fifa_world_cup_2006 GPU_PLUGINS xenos)


if(CMAKE_SYSTEM_PROCESSOR MATCHES "AMD64|x86_64")
    target_compile_options(fifa_world_cup_2006_recomp PRIVATE /clang:-march=x86-64-v2)
endif()
