vcpkg_check_linkage(ONLY_STATIC_LIBRARY)

vcpkg_from_github(
    OUT_SOURCE_PATH SOURCE_PATH
    REPO ArtifexSoftware/mupdf
    REF "${VERSION}"
    SHA512 76b0cd9ea2ea203ae37ce17dd2de8fceb13d0ff2fc24a31483306c6a6ecd4851d2ecb81e177c3ebc1e4eea7404697b4aeba451883dd40b21f10bb3eb101da411
    HEAD_REF master
    PATCHES
        dont-generate-extract-3rd-party-things.patch
        fix-NAN-on-Win11.patch # https://github.com/ArtifexSoftware/mupdf/pull/54
)

file(COPY "${CMAKE_CURRENT_LIST_DIR}/CMakeLists.txt" DESTINATION "${SOURCE_PATH}")

# MuPDF links against the HarfBuzz TeXyoN bundles, so a binary holds one copy.
# vcpkg's ABI hash covers this file but not those headers, so a submodule bump
# must change the pin below to rebuild MuPDF.
set(BUNDLED_HARFBUZZ_VERSION "10.2.0")
set(BUNDLED_HARFBUZZ_INCLUDE "${CMAKE_CURRENT_LIST_DIR}/../../../crates/bridge_harfbuzz/harfbuzz/src")
file(STRINGS "${BUNDLED_HARFBUZZ_INCLUDE}/hb-version.h" HB_VERSION_LINE REGEX "^#define HB_VERSION_STRING ")
if(NOT HB_VERSION_LINE MATCHES "\"${BUNDLED_HARFBUZZ_VERSION}\"")
    message(FATAL_ERROR "bundled HarfBuzz is not ${BUNDLED_HARFBUZZ_VERSION} (${HB_VERSION_LINE}), update the pin")
endif()

vcpkg_check_features(
    OUT_FEATURE_OPTIONS FEATURE_OPTIONS
    FEATURES
        ocr ENABLE_OCR
)

vcpkg_cmake_configure(
    SOURCE_PATH "${SOURCE_PATH}"
    DISABLE_PARALLEL_CONFIGURE
    OPTIONS
        -DBUILD_EXAMPLES=OFF
        "-DHARFBUZZ_INCLUDE=${BUNDLED_HARFBUZZ_INCLUDE}"
        ${FEATURE_OPTIONS}
)

vcpkg_cmake_install()

file(COPY "${SOURCE_PATH}/include/mupdf" DESTINATION "${CURRENT_PACKAGES_DIR}/include")

vcpkg_copy_pdbs()

vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/COPYING")
