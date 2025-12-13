cmake_minimum_required(VERSION 3.22.1)
project(taigi_trie)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 支援 16 KB page size（Google Play 2025/11 起要求）
set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -Wl,-z,max-page-size=16384")

# MARISA-trie source files
set(MARISA_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/marisa-src)

# MARISA library sources
set(MARISA_SOURCES
    ${MARISA_SRC_DIR}/lib/marisa/agent.cc
    ${MARISA_SRC_DIR}/lib/marisa/keyset.cc
    ${MARISA_SRC_DIR}/lib/marisa/trie.cc
    ${MARISA_SRC_DIR}/lib/marisa/grimoire/io/mapper.cc
    ${MARISA_SRC_DIR}/lib/marisa/grimoire/io/reader.cc
    ${MARISA_SRC_DIR}/lib/marisa/grimoire/io/writer.cc
    ${MARISA_SRC_DIR}/lib/marisa/grimoire/trie/louds-trie.cc
    ${MARISA_SRC_DIR}/lib/marisa/grimoire/trie/tail.cc
    ${MARISA_SRC_DIR}/lib/marisa/grimoire/vector/bit-vector.cc
)

# Build MARISA as static library
add_library(marisa STATIC ${MARISA_SOURCES})

target_include_directories(marisa PUBLIC
    ${MARISA_SRC_DIR}/include
    ${MARISA_SRC_DIR}/lib
)

# JNI wrapper library
add_library(taigi_trie SHARED
    trie_jni.cpp
)

target_include_directories(taigi_trie PRIVATE
    ${MARISA_SRC_DIR}/include
)

target_link_libraries(taigi_trie
    marisa
    log
)
