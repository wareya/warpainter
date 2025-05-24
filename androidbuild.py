#!/usr/bin/env python
import os
import subprocess
import shutil
import struct
import zipfile
import sys
from datetime import datetime
import json
import requests

# Assuming mvn_resolve.py is in the same directory
# This assumes mvn_resolve.py exists and provides a get_dependencies_as_json function
try:
    from mvn_resolve import get_dependencies_as_json
except ImportError:
    print("Error: mvn_resolve.py not found. Please ensure it's in the same directory as build.py.")
    print("You can get it from the previous responses or create it if you haven't already.")
    sys.exit(1)


# --- User-Editable Configuration ---
# Set ANDROID_HOME to your Android SDK installation directory.
# On Windows, this is typically C:\Users\<YourUser>\AppData\Local\Android\Sdk
# On Linux/macOS, it might be ~/Android/Sdk or /opt/android-sdk
# The script attempts to determine it for Windows, but you can override it here.
# Example for Windows: ANDROID_HOME_VAR = r"C:\Users\YourUser\AppData\Local\Android\Sdk"
# Example for Linux/macOS: ANDROID_HOME_VAR = os.path.expanduser("~/Android/Sdk")
ANDROID_HOME_VAR = None # Set to None to let the script try to determine it, or provide a string path.

SDK_VER = "35.0.1" # Android SDK Build-Tools version (e.g., "35.0.1")
ANDROID_PLATFORM_VERSION = "android-35" # Android platform version for compilation (e.g., "android-35")

# Path to your debug keystore.
# This is typically ~/.android/debug.keystore
DEBUG_KEYSTORE_PATH = os.path.expanduser("~/.android/debug.keystore")
# --- End User-Editable Configuration ---

# --- Maven Dependency Configuration ---
MAVEN_DEPENDENCIES = [
    "androidx.activity:activity:1.9.3", # Reverted to 1.9.3 as per user feedback
    "com.google.guava:listenablefuture:1.0", # Explicitly added for ListenableFuture
]
MAVEN_REPOSITORIES = [
    "https://repo.maven.apache.org/maven2/", # Prioritize Maven Central
    "https://maven.google.com/",
]
# --- End Maven Dependency Configuration ---

# --- External Manifest Merger Tool Configuration ---
DISTRIQT_MANIFEST_MERGER_URL = "https://github.com/distriqt/android-manifest-merger/releases/download/v31.9.0/manifest-merger-31.9.0.jar"
DISTRIQT_MANIFEST_MERGER_FILENAME = "manifest-merger-31.9.0.jar"
# --- End External Manifest Merger Tool Configuration ---


def run_command(cmd, cwd=None, check_output=False):
    """
    Executes a shell command, prints it, and handles errors.
    Args:
        cmd (list): The command and its arguments as a list.
        cwd (str, optional): The current working directory for the command. Defaults to None.
        check_output (bool, optional): If True, returns stdout. Defaults to False.
    Returns:
        str: The stdout if check_output is True, otherwise None.
    Raises:
        subprocess.CalledProcessError: If the command returns a non-zero exit code.
    """
    print(f"\nExecuting command: {' '.join(cmd)}")
    try:
        if check_output:
            result = subprocess.run(cmd, cwd=cwd, check=True, text=True, capture_output=True)
            print(result.stdout)
            if result.stderr:
                print(result.stderr)
            return result.stdout
        else:
            subprocess.run(cmd, cwd=cwd, check=True)
    except subprocess.CalledProcessError as e:
        print(f"Error executing command: {e}")
        if e.stdout:
            print(f"Stdout:\n{e.stdout}")
        if e.stderr:
            print(f"Stderr:\n{e.stderr}")
        sys.exit(1) # Exit the script on command failure
    except FileNotFoundError:
        print(f"Error: Command not found. Make sure '{cmd[0]}' is in your PATH or correctly specified.")
        sys.exit(1)

def patch_zip_timestamps(file_path):
    """
    Patches timestamps inside a ZIP file to make it deterministic (e.g., for APKs/JARs).
    This function mimics the shell script's `patch_zip_timestamps` function.
    Args:
        file_path (str): The path to the ZIP file (e.g., .jar or .apk).
    """
    print(f"Deterministicifying zip/jar: {file_path}...")
    # ZIP local file header signature: 0x04034b50 (PK\x03\x04)
    # ZIP central directory file header signature: 0x02014b50 (PK\x01\x02)

    # Offsets for modification:
    # Local file header: general purpose bit flag (2 bytes) + compression method (2 bytes) +
    #                    last mod file time (2 bytes) + last mod file date (2 bytes)
    #                    -> time/date are at offset 10 from signature.
    # Central directory file header: version made by (2 bytes) + version needed (2 bytes) +
    #                                general purpose bit flag (2 bytes) + compression method (2 bytes) +
    #                                last mod file time (2 bytes) + last mod file date (2 bytes)
    #                                -> time/date are at offset 12 from signature.

    # We want to zero out the 4 bytes representing time and date.
    # The shell script uses 0x504b0304 (PK\x03\x04) and 0x504b0102 (PK\x01\x02)
    # These are little-endian representations of the signatures.

    # Signatures as byte strings (little-endian)
    local_file_header_sig = b'\x50\x4b\x03\x04'
    central_dir_header_sig = b'\x50\x4b\x01\x02'

    # (signature length + offset to time/date field, total size of header)
    patches = [
        (local_file_header_sig, 10, 30), # (sig, offset_to_time_date, total_header_length)
        (central_dir_header_sig, 12, 46) # (sig, offset_to_time_date, total_header_length)
    ]

    with open(file_path, 'r+b') as f:
        content = bytearray(f.read())

        for sig, offset_to_time_date, total_header_length in patches:
            current_offset = 0
            while True:
                # Find the signature
                pos = content.find(sig, current_offset)
                if pos == -1:
                    break

                # Zero out the 4 bytes for time and date
                # The `dd` command in the shell script writes 4 null bytes.
                for i in range(4):
                    if pos + offset_to_time_date + i < len(content):
                        content[pos + offset_to_time_date + i] = 0x00

                # Move current_offset past this header to find the next one
                current_offset = pos + total_header_length

        f.seek(0)
        f.write(content)
        f.truncate() # Ensure no extra bytes if content became shorter (unlikely here)

    print("Deterministicified.")

def download_file(url, destination_path):
    """Downloads a file from a URL to a specified path."""
    if os.path.exists(destination_path):
        print(f"File already exists: {destination_path}. Skipping download.")
        return True

    print(f"Downloading {url} to {destination_path}...")
    try:
        with requests.get(url, stream=True, timeout=30) as r:
            r.raise_for_status()
            with open(destination_path, 'wb') as f:
                for chunk in r.iter_content(chunk_size=8192):
                    f.write(chunk)
        print("Download complete.")
        return True
    except requests.exceptions.RequestException as e:
        print(f"Error downloading {url}: {e}", file=sys.stderr)
        return False

def ensure_dir_exists(path):
    """
    Ensures a directory exists at the given path.
    If a file exists at this path, it will be removed before creating the directory.
    This also handles the Windows FileExistsError (WinError 183) that can occur
    even when the target path is already a directory.
    """
    if os.path.isfile(path):
        # A file exists where a directory is needed. This is a critical conflict.
        print(f"ERROR: A file was found at '{path}' where a directory is expected. Removing the conflicting file.", file=sys.stderr)
        try:
            os.remove(path)
        except OSError as e:
            print(f"Error: Failed to remove conflicting file '{path}': {e}", file=sys.stderr)
            sys.exit(1) # This is unrecoverable if we can't remove the blocker
    
    try:
        os.makedirs(path, exist_ok=True)
    except FileExistsError as e:
        if os.path.isdir(path):
            # This is the Windows quirk: directory exists, but FileExistsError was still raised.
            print(f"Warning: Directory '{path}' already exists, but os.makedirs still raised FileExistsError (WinError 183). Proceeding.", file=sys.stderr)
        else:
            # For any other unexpected FileExistsError scenarios (e.g., symlink to non-existent target)
            print(f"Error: An unexpected FileExistsError occurred for '{path}': {e}", file=sys.stderr)
            sys.exit(1)
    except Exception as e:
        # Catch any other potential errors during directory creation (e.g., permission denied)
        print(f"Error creating directory '{path}': {e}", file=sys.stderr)
        sys.exit(1)


def extract_aar(aar_path, extract_dir_for_jars, lib_output_dir, res_output_dir, manifest_output_dir):
    """
    Extracts contents of an AAR file.
    Specifically extracts classes.jar, AndroidManifest.xml, res/, and jni/.
    Handles directory entries correctly.
    """
    print(f"Extracting AAR: {aar_path}")

    extracted_jar_path = None
    extracted_manifest_path = None
    extracted_res_dir = None

    with zipfile.ZipFile(aar_path, 'r') as zip_ref:
        for member in zip_ref.namelist():
            if member == 'classes.jar':
                jar_name = os.path.basename(aar_path).replace(".aar", "-classes.jar")
                extracted_jar_path = os.path.join(extract_dir_for_jars, jar_name)
                print(f"Extracting classes.jar to {extracted_jar_path}")
                with open(extracted_jar_path, 'wb') as outfile:
                    outfile.write(zip_ref.read(member))
            elif member == 'AndroidManifest.xml':
                manifest_name = os.path.basename(aar_path).replace(".aar", "_AndroidManifest.xml")
                extracted_manifest_path = os.path.join(manifest_output_dir, manifest_name)
                print(f"Extracting AndroidManifest.xml to {extracted_manifest_path}")
                with open(extracted_manifest_path, 'wb') as outfile:
                    outfile.write(zip_ref.read(member))
            elif member.startswith('res/') and len(member) > 4: # Ensure it's a file or non-empty directory entry
                # Determine the target path on disk
                target_path_on_disk = os.path.join(res_output_dir, os.path.relpath(member, 'res'))

                if member.endswith('/'): # This is a directory entry in the zip (e.g., 'res/values/')
                    # Ensure this directory exists
                    ensure_dir_exists(target_path_on_disk)
                    print(f"Ensuring directory exists: {target_path_on_disk}")
                else: # This is a file entry (e.g., 'res/values/styles.xml')
                    target_dir = os.path.dirname(target_path_on_disk)
                    # Ensure the parent directory for the resource file exists
                    ensure_dir_exists(target_dir)

                    print(f"Extracting resource file to {target_path_on_disk}")
                    with open(target_path_on_disk, 'wb') as outfile:
                        outfile.write(zip_ref.read(member))
                    extracted_res_dir = res_output_dir # Mark that resources were extracted
            elif member.startswith('jni/') and len(member) > 4: # Ensure it's a file or non-empty directory entry
                target_path_on_disk = os.path.join(lib_output_dir, os.path.relpath(member, 'jni'))

                if member.endswith('/'): # This is a directory entry in the zip (e.g., 'jni/armeabi-v7a/')
                    ensure_dir_exists(target_path_on_disk)
                    print(f"Ensuring directory exists: {target_path_on_disk}")
                else: # This is a file entry (e.g., 'jni/armeabi-v7a/libfoo.so')
                    target_dir = os.path.dirname(target_path_on_disk)
                    ensure_dir_exists(target_dir)

                    print(f"Extracting native library to {target_path_on_disk}")
                    with open(target_path_on_disk, 'wb') as outfile:
                        outfile.write(zip_ref.read(member))

    print(f"Finished extracting AAR: {aar_path}")
    return extracted_jar_path, extracted_manifest_path, extracted_res_dir

def main():
    # --- Determine ANDROID_HOME ---
    ANDROID_HOME = ANDROID_HOME_VAR
    if ANDROID_HOME is None:
        if sys.platform.startswith('win'):
            user_profile = os.environ.get('USERPROFILE')
            if not user_profile:
                print("Error: USERPROFILE environment variable not found.")
                sys.exit(1)
            ANDROID_HOME = os.path.join(user_profile, 'AppData', 'Local', 'Android', 'Sdk')
        else:
            # Fallback for Linux/macOS if not explicitly set
            ANDROID_HOME = os.environ.get('ANDROID_HOME')
            if not ANDROID_HOME:
                print("Error: ANDROID_HOME environment variable not set and not provided in script. Please set it or configure ANDROID_HOME_VAR.")
                sys.exit(1)

    # Paths to SDK tools
    AAPT2 = os.path.join(ANDROID_HOME, 'build-tools', SDK_VER, 'aapt2')
    D8_JAR = os.path.join(ANDROID_HOME, 'build-tools', SDK_VER, 'lib', 'd8.jar')
    APKSIGNER_JAR = os.path.join(ANDROID_HOME, 'build-tools', SDK_VER, 'lib', 'apksigner.jar')
    ZIPALIGN = os.path.join(ANDROID_HOME, 'build-tools', SDK_VER, 'zipalign')
    
    # --- Build Directories (Strict Separation) ---
    BUILD_DIR = ".androidbuild"
    # Temporary files: downloaded jars, compiled .class, classes.dex
    BUILD_TMP_DIR = os.path.join(BUILD_DIR, "tmp") 
    # Base directory for compiled flat resources from AARs (each AAR gets a sub-dir)
    BUILD_AAR_COMPILED_RES_DIR_BASE = os.path.join(BUILD_DIR, "aapt2_output", "aar_res_compiled")
    # Compiled flat resources specifically for the app's own resources
    BUILD_APP_RES_COMPILED_DIR = os.path.join(BUILD_DIR, "aapt2_output", "app_res_compiled")
    # Generated R.java file
    BUILD_R_JAVA_DIR = os.path.join(BUILD_DIR, "aapt2_output", "r_java")
    # Raw resources extracted from AARs (each AAR gets a sub-dir here)
    BUILD_AAR_RES_SRC_DIR_BASE = os.path.join(BUILD_DIR, "extracted_aars", "res_src")
    # AndroidManifest.xml files extracted from AARs
    BUILD_AAR_MANIFEST_DIR = os.path.join(BUILD_DIR, "extracted_aars", "manifests")
    # Directory for the merged AndroidManifest.xml
    BUILD_MERGED_MANIFEST_DIR = os.path.join(BUILD_DIR, "merged_manifests")
    # Native libraries (from cargo-ndk and AARs)
    BUILD_NATIVE_LIBS_DIR = os.path.join(BUILD_DIR, "native_libs")
    # Maven's POM cache
    POM_CACHE_DIR = ".pom_cache" # This is usually outside the build dir, can be global or project-local.

    TARGET_DIR = "target" # Final APK output directory

    # Output APK names
    UNSIGNED_APK = os.path.join(TARGET_DIR, "warpainter-unsigned.apk")
    ALIGNED_APK = os.path.join(TARGET_DIR, "warpainter-aligned.apk")
    SIGNED_APK = os.path.join(TARGET_DIR, "warpainter-signed.apk")

    # Source directories (should not be polluted by build output)
    ANDROID_SRC_ROOT = "android" # Your app's Android source root
    APP_RES_DIR = os.path.join(ANDROID_SRC_ROOT, "res") # Your app's own resources
    APP_MANIFEST_PATH = os.path.join(ANDROID_SRC_ROOT, "AndroidManifest.xml")
    APP_JAVA_SRC_PATH = os.path.join("src", "FileOpenActivity.java") # Assuming 'src' is also a source dir at root
    APP_ASSETS_DIR = os.path.join(ANDROID_SRC_ROOT, "assets") # Your app's own assets (e.g., android/assets)

    # NEW: Path to the distriqt manifest-merger.jar
    MANIFEST_MERGER_JAR = os.path.join(BUILD_TMP_DIR, DISTRIQT_MANIFEST_MERGER_FILENAME)


    ANDROID_JAR = os.path.join(ANDROID_HOME, 'platforms', ANDROID_PLATFORM_VERSION, 'android.jar')

    # --- Pre-requisite Checks ---
    if not os.path.exists(ANDROID_HOME):
        print(f"Error: ANDROID_HOME path does not exist: {ANDROID_HOME}")
        sys.exit(1)
    if not os.path.exists(ANDROID_JAR):
        print(f"Error: Android platform JAR not found: {ANDROID_JAR}. Ensure SDK {ANDROID_PLATFORM_VERSION} is installed.")
        sys.exit(1)
    if not os.path.exists(DEBUG_KEYSTORE_PATH):
        print(f"Warning: Debug keystore not found at {DEBUG_KEYSTORE_PATH}. APKSIGNER step might fail.")
        print("You can generate one using: keytool -genkeypair -v -keystore ~/.android/debug.keystore -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 -dname \"CN=Android Debug,O=Android,C=US\" -storepass android -keypass android\"")
    # No longer checking for the SDK's internal manifest-merger.jar here, as we download an external one.


    # --- Ensure necessary build directories exist ---
    print("Ensuring necessary build directories exist...")
    dirs_to_ensure = [
        BUILD_TMP_DIR,
        BUILD_AAR_COMPILED_RES_DIR_BASE, # Base directory for compiled AAR resources
        BUILD_APP_RES_COMPILED_DIR, # Dedicated for app's resources
        BUILD_R_JAVA_DIR,
        BUILD_AAR_RES_SRC_DIR_BASE, # Base directory for AAR raw resources
        BUILD_AAR_MANIFEST_DIR,
        BUILD_MERGED_MANIFEST_DIR,
        BUILD_NATIVE_LIBS_DIR,
        POM_CACHE_DIR,
        TARGET_DIR
    ]
    for d in dirs_to_ensure:
        ensure_dir_exists(d) # Using the robust helper function

    # --- Download External Manifest Merger JAR ---
    print(f"Downloading external manifest merger JAR from {DISTRIQT_MANIFEST_MERGER_URL}...")
    if not download_file(DISTRIQT_MANIFEST_MERGER_URL, MANIFEST_MERGER_JAR):
        print(f"Error: Failed to download external manifest merger JAR from {DISTRIQT_MANIFEST_MERGER_URL}. Cannot proceed.")
        sys.exit(1)
    print(f"Using external manifest merger: {MANIFEST_MERGER_JAR}")


    # --- Maven Dependency Resolution and Download ---
    print("Resolving Maven dependencies and downloading AARs/JARs...")
    resolved_deps = get_dependencies_as_json(
        initial_dependencies=MAVEN_DEPENDENCIES,
        repo_urls=MAVEN_REPOSITORIES,
        scopes_to_include=["compile", "runtime"],
        #check_artifact_existence=True, # slow. no longer needed
        pom_cache_dir=POM_CACHE_DIR,
        blacklist_gav_keys=[
            "org.jetbrains.kotlin:kotlin-stdlib-jdk7",
            "org.jetbrains.kotlin:kotlin-stdlib-jdk8",
        ],
    )

    if resolved_deps.get("error"):
        print(f"Error resolving Maven dependencies: {resolved_deps['error']}", file=sys.stderr)
        sys.exit(1)
    if resolved_deps.get("warnings"):
        for warning in resolved_deps["warnings"]:
            print(warning, file=sys.stderr)


    aar_extracted_jars = []
    aar_manifests_to_merge = [] # Manifests from AARs

    for dep in resolved_deps["dependencies"]:
        gav_key = f"{dep['groupId']}:{dep['artifactId']}:{dep['version']}"
        
        # Skip POM-packaged dependencies as they are metadata, not binary artifacts to download directly
        if dep["packaging"] == "pom":
            print(f"INFO: Skipping POM-packaged dependency: {gav_key}")
            continue

        if not dep["download_url"]:
            print(f"ERROR: No download URL for {dep['packaging']} {gav_key}. Exiting.", file=sys.stderr)
            sys.exit(1) # Hard error for missing download URL
        
        # Determine file extension based on packaging
        file_extension = "jar" if dep["packaging"] == "jar" else "aar" if dep["packaging"] == "aar" else None
        if not file_extension:
            print(f"ERROR: Unknown packaging type '{dep['packaging']}' for {gav_key}. Exiting.", file=sys.stderr)
            sys.exit(1) # Hard error for unknown packaging

        local_artifact_filename = f"{dep['artifactId']}-{dep['version']}.{file_extension}"
        local_artifact_path = os.path.join(BUILD_TMP_DIR, local_artifact_filename) # Download to build/tmp
        
        if not download_file(dep["download_url"], local_artifact_path):
            print(f"ERROR: Failed to download {dep['packaging']} from {dep['download_url']}. Exiting.", file=sys.stderr)
            sys.exit(1) # Hard error for download failure
        
        if dep["packaging"] == "aar":
            # Create a unique directory for this AAR's raw resources
            current_aar_res_src_dir = os.path.join(BUILD_AAR_RES_SRC_DIR_BASE, f"{dep['artifactId']}-{dep['version']}")
            ensure_dir_exists(current_aar_res_src_dir) # Ensure this unique directory exists

            # Create a unique directory for this AAR's *compiled* resources
            current_aar_compiled_res_dir = os.path.join(BUILD_AAR_COMPILED_RES_DIR_BASE, f"{dep['artifactId']}-{dep['version']}")
            ensure_dir_exists(current_aar_compiled_res_dir)

            # Extract AAR contents
            jar_path, manifest_path, res_dir = extract_aar(
                local_artifact_path,
                BUILD_TMP_DIR,          # extracted AAR jars go here
                BUILD_NATIVE_LIBS_DIR,  # native libs go here
                current_aar_res_src_dir, # Pass the UNIQUE directory for this AAR's raw resources
                BUILD_AAR_MANIFEST_DIR  # AAR manifests extracted here
            )
            if jar_path:
                aar_extracted_jars.append(jar_path)
            if manifest_path:
                aar_manifests_to_merge.append(manifest_path)
            if res_dir:
                # Compile AAR resources from their *unique* extracted source directory
                # Output to their *unique* compiled resource directory
                print(f"Compiling AAR resources from {res_dir} to {current_aar_compiled_res_dir}...")
                run_command([AAPT2, "compile", "--dir", res_dir, "-o", current_aar_compiled_res_dir])

        elif dep["packaging"] == "jar":
            aar_extracted_jars.append(local_artifact_path) # Treat as an extracted JAR for D8


    # --- Rust/Cargo NDK Build ---
    print("Building Rust project with cargo-ndk...")
    run_command(["cargo", "ndk", "-t", "arm64-v8a", "-o", BUILD_NATIVE_LIBS_DIR, "build", "--release"])
    # Reset terminal color/formatting after cargo-ndk might have changed it
    print("\033[0m", end='') 

    # --- Manifest Merging ---
    print("Merging AndroidManifest.xml files using distriqt manifest-merger...")
    merged_manifest_path = os.path.join(BUILD_MERGED_MANIFEST_DIR, "AndroidManifest.xml")
    
    manifest_merger_cmd = [
        "java", "-jar", MANIFEST_MERGER_JAR,
        "--main", APP_MANIFEST_PATH, # Your app's main manifest
        "--out", merged_manifest_path # Output merged merged_manifest_path
    ]
    
    # Add all extracted AAR manifests as library manifests
    if aar_manifests_to_merge:
        # Use os.pathsep to get the correct path separator for the OS (; on Windows, : on Linux/macOS)
        libs_path_list = os.pathsep.join(aar_manifests_to_merge)
        manifest_merger_cmd.extend(["--libs", libs_path_list])
        
    run_command(manifest_merger_cmd)


    # --- Compile App's own Android Resources ---
    print("Compiling app's own Android resources...")
    # `aapt2 compile` for app's resources now outputs to its dedicated directory.
    run_command([AAPT2, "compile", "--dir", APP_RES_DIR, "-o", BUILD_APP_RES_COMPILED_DIR])
    
    # --- Link Android Resources and Create Unsigned APK ---
    print("Linking Android resources and creating unsigned APK...")
    
    # Get all compiled resource files: app's own and AARs' in specific order
    all_compiled_res_files_for_linking = []
    # Add app's compiled resources first
    all_compiled_res_files_for_linking.extend([os.path.join(BUILD_APP_RES_COMPILED_DIR, f) for f in os.listdir(BUILD_APP_RES_COMPILED_DIR) if os.path.isfile(os.path.join(BUILD_APP_RES_COMPILED_DIR, f)) and f.endswith('.flat')])
    
    # Then add ALL AAR compiled resources from their unique compiled directories
    # Iterate through each unique AAR compiled resource directory
    for aar_compiled_dir_name in os.listdir(BUILD_AAR_COMPILED_RES_DIR_BASE):
        full_aar_compiled_dir = os.path.join(BUILD_AAR_COMPILED_RES_DIR_BASE, aar_compiled_dir_name)
        if os.path.isdir(full_aar_compiled_dir): # Ensure it's actually a directory
            all_compiled_res_files_for_linking.extend([os.path.join(full_aar_compiled_dir, f) for f in os.listdir(full_aar_compiled_dir) if os.path.isfile(os.path.join(full_aar_compiled_dir, f)) and f.endswith('.flat')])


    aapt2_link_cmd = [
        AAPT2, "link",
        "-I", ANDROID_JAR,
        "--manifest", merged_manifest_path, # USE THE MERGED MANIFEST HERE
        "-o", UNSIGNED_APK,
        "--java", BUILD_R_JAVA_DIR, # Generates R.java into build/aapt2_output/r_java
        "--extra-packages", "androidx.startup", # Explicitly generate R.java for androidx.startup package
    ] + all_compiled_res_files_for_linking # Use the new list for linking
    
    # If your app uses assets, add them to the link command.
    if os.path.exists(APP_ASSETS_DIR) and os.listdir(APP_ASSETS_DIR):
        aapt2_link_cmd.extend(["--assets", APP_ASSETS_DIR])
    
    run_command(aapt2_link_cmd)

    # --- Diagnostic: R.java files generated by aapt2 link in BUILD_R_JAVA_DIR ---
    print(f"\n--- Diagnostic: R.java files generated by aapt2 link in {BUILD_R_JAVA_DIR} ---")
    found_startup_r_java = False
    for root, _, files in os.walk(BUILD_R_JAVA_DIR):
        for file in files:
            if file.endswith('.java'):
                full_java_path = os.path.join(root, file)
                print(f"  - {full_java_path}")
                if "androidx" in full_java_path and "startup" in full_java_path and "R.java" in file:
                    found_startup_r_java = True
    if not found_startup_r_java:
        print("  CRITICAL: androidx/startup/R.java NOT found in aapt2 link output. This is why the class is missing.")
    print("-------------------------------------------------------------------\n")


    # --- Java Compilation ---
    print("Compiling Java sources (app and generated R.java)...")
    
    # On Windows, classpath uses ';' as separator, on Linux/macOS it's ':'
    classpath_separator = ';' if sys.platform.startswith('win') else ':' # Moved definition here
    # All JARs for classpath (your own + extracted AAR JARs)
    classpath_jars = [os.path.join(BUILD_TMP_DIR, f) for f in os.listdir(BUILD_TMP_DIR) if f.endswith('.jar') or f.endswith('classes.jar')]
    classpath = f"{ANDROID_JAR}{classpath_separator}{classpath_separator.join(classpath_jars)}"

    # Collect all Java source files to compile: app's main source + all generated R.java files
    java_sources_to_compile = [APP_JAVA_SRC_PATH]
    for root, _, files in os.walk(BUILD_R_JAVA_DIR):
        for file in files:
            if file.endswith('.java'):
                java_sources_to_compile.append(os.path.join(root, file))

    print(f"\n--- Diagnostic: Java source files being passed to javac ---")
    for src_file in java_sources_to_compile:
        print(f"  - {src_file}")
    print("-----------------------------------------------------------\n")

    javac_cmd = [
        "javac",
        "-bootclasspath", ANDROID_JAR,
        "-classpath", classpath, # This classpath is for resolving dependencies *during* compilation
        "-d", BUILD_TMP_DIR, # Compile all Java classes directly into build/tmp
        "-target", "1.8",
        "-source", "1.8"
    ] + java_sources_to_compile # Add all collected Java source files
    run_command(javac_cmd)

    print(f"\n--- Diagnostic: .class files generated by javac in {BUILD_TMP_DIR} ---")
    found_app_r_class = False
    found_app_r_string_class = False
    for root, _, files in os.walk(BUILD_TMP_DIR):
        for file in files:
            if file.endswith('.class'):
                full_class_path = os.path.join(root, file)
                print(f"  - {full_class_path}")
                if "R.class" in full_class_path and "moe" in full_class_path and "wareya" in full_class_path and "warpainter" in full_class_path: # Look for app's R.class
                    found_app_r_class = True
                if "R$string.class" in full_class_path and "moe" in full_class_path and "wareya" in full_class_path and "warpainter" in full_class_path: # Look for app's R$string.class
                    found_app_r_string_class = True

    if not found_app_r_class:
        print("  WARNING: Application's R.class not found in javac output directory.")
    if not found_app_r_string_class:
        print("  WARNING: Application's R$string.class not found in javac output directory.")
    print("-------------------------------------------------------------------\n")


    # --- D8 (Dexer) Compilation ---
    print("Dexing Java classes with D8...")
    # Find all compiled .class files recursively within BUILD_TMP_DIR (your own classes)
    compiled_local_class_files = []
    for root, _, files in os.walk(BUILD_TMP_DIR):
        for file in files:
            if file.endswith('.class'):
                compiled_local_class_files.append(os.path.join(root, file))

    d8_input_jars_and_classes = compiled_local_class_files + aar_extracted_jars # All .class files + all dependency JARs

    print(f"\n--- Diagnostic: Input files for D8 ---")
    # Limit output for brevity if list is very long
    for d8_input in d8_input_jars_and_classes:
        print(f"  - {d8_input}")
    print("--------------------------------------\n")

    d8_cmd = [
        "java",
        "-cp", D8_JAR,
        "com.android.tools.r8.D8",
        "--output", BUILD_TMP_DIR, # D8 output (classes.dex) goes into build/tmp
    ] + d8_input_jars_and_classes + [ # Pass all found .class files and .jar files
        "--min-api", "30"
    ]
    run_command(d8_cmd)

    # --- Deterministic Timestamp for classes.dex ---
    print("Setting deterministic timestamp for classes.dex...")
    classes_dex_path = os.path.join(BUILD_TMP_DIR, "classes.dex")
    epoch_1980_01_01 = datetime(1980, 1, 1, 0, 0, 0).timestamp()
    os.utime(classes_dex_path, (epoch_1980_01_01, epoch_1980_01_01))

    # --- Add classes.dex and Native Libraries to APK ---
    print("Adding classes.dex and native libraries to unsigned APK...")
    with zipfile.ZipFile(UNSIGNED_APK, 'a', zipfile.ZIP_DEFLATED) as apk_zip:
        # Add classes.dex directly to the root of the APK (uncompressed)
        apk_zip.write(classes_dex_path, arcname="classes.dex", compress_type=zipfile.ZIP_STORED)

        # Add native libraries (from cargo-ndk and AARs)
        # These are now in BUILD_NATIVE_LIBS_DIR
        for root, _, files in os.walk(BUILD_NATIVE_LIBS_DIR):
            for file in files:
                full_path = os.path.join(root, file)
                # arcname should be relative to the APK root, e.g., lib/arm64-v8a/libfoo.so
                # Use os.path.relpath from BUILD_NATIVE_LIBS_DIR to get the correct APK path
                relative_path = os.path.relpath(full_path, BUILD_NATIVE_LIBS_DIR)
                # Native libraries should always be ZIP_STORED (uncompressed)
                print(f"Adding native library to APK: lib/{relative_path}")
                apk_zip.write(full_path, arcname=os.path.join("lib", relative_path), compress_type=zipfile.ZIP_STORED)

    # --- Zipalign APK ---
    print("Zipaligning APK...")
    # It's typical to remove the target file if it exists before zipalign creates a new one.
    if os.path.exists(ALIGNED_APK):
        os.remove(ALIGNED_APK)
    run_command([ZIPALIGN, "-v", "4", UNSIGNED_APK, ALIGNED_APK])

    # --- Sign APK ---
    print("Signing APK...")
    run_command([
        "java", "-jar", APKSIGNER_JAR, "sign",
        "--ks", DEBUG_KEYSTORE_PATH,
        "--ks-key-alias", "androiddebugkey",
        "--ks-pass", "pass:android",
        "--key-pass", "pass:android",
        "--out", SIGNED_APK,
        ALIGNED_APK
    ])

    # --- Install APK (Optional) ---
    print("Installing APK...")
    run_command(["adb", "install", SIGNED_APK])

    # --- Logcat (Optional) ---
    print("\nStarting adb logcat (Ctrl+C to stop)...")
    # Clear logcat buffer
    run_command(["adb", "logcat", "-c"])
    # Start the activity and then stream logcat
    # Use Popen directly for non-blocking start of the activity on the host via shell
    start_activity_cmd_string = "adb shell am start -n moe.wareya.warpainter/android.app.NativeActivity"
    print(f"\nExecuting command (non-blocking): {start_activity_cmd_string} &") # Print the command with '&'
    try:
        # Use shell=True and append '&' to the command string to run it in the background on the host shell
        subprocess.Popen(
            f"{start_activity_cmd_string} &",
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, shell=True
        )
    except Exception as e:
        print(f"Error starting activity: {e}", file=sys.stderr)
        # Do not exit, try to proceed to logcat anyway in case it's a minor issue

    # Use Popen to stream logcat output
    try:
        print("Attempting to stream logcat output...")
        # The logcat command string for piping in shell, with --line-buffered for grep
        logcat_command_string = "adb logcat | grep --line-buffered -iP \"moe.wareya.warpainter| rust|[\\w]System|FileOpen|[ \\t]E[ \\t]|NativeContext\""
        logcat_process = subprocess.Popen(
            logcat_command_string,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, shell=True
        )
        for line in logcat_process.stdout:
            print(line, end='', flush=True) # Added flush=True and kept original end=''
        
        # Check for any stderr output if the process terminates
        stderr_output = logcat_process.stderr.read()
        if stderr_output:
            print("\n--- Logcat Stderr Output ---", file=sys.stderr)
            print(stderr_output, file=sys.stderr)
            print("----------------------------", file=sys.stderr)

        logcat_process.wait()
    except KeyboardInterrupt:
        print("\nLogcat stopped by user.")
    except Exception as e:
        print(f"Error during logcat: {e}")

    print("\nAndroid build and installation process completed.")

if __name__ == "__main__":
    main()
