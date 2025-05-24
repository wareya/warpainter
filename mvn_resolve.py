#!/usr/bin/env python
import requests
import xml.etree.ElementTree as ET
import os
from typing import List, Dict, Set, Optional, Tuple
from collections import deque
import re
import json
import sys

class MavenDependencyAnalyzer:
    """
    Analyzes Maven transitive dependencies by directly fetching and parsing
    POM files from a list of Maven repositories. It implements a "highest version wins"
    mediation strategy for conflicting groupId:artifactId versions, and provides
    download URLs for the resolved artifacts. It also tracks the first direct
    depender for each resolved dependency.
    """

    def __init__(self, repository_urls: List[str],
                 include_scopes: List[str],
                 ignore_optional_dependencies: bool = True,
                 exclude_runtime_dependencies: bool = False,
                 blacklist_gav_keys: Optional[List[str]] = None,
                 pom_cache_dir: Optional[str] = None,
                 check_artifact_existence: bool = False): # NEW: Control artifact existence check
        
        self.repository_urls = [url.rstrip('/') + '/' for url in repository_urls]
        self.include_scopes = set(include_scopes)

        if exclude_runtime_dependencies:
            if "runtime" in self.include_scopes:
                self.include_scopes.remove("runtime")

        self.ignore_optional_dependencies = ignore_optional_dependencies
        self.pom_cache: Dict[str, str] = {} # In-memory cache for current run
        self.resolved_artifacts: Dict[str, Dict[str, str]] = {}
        self.queued_gavs: Set[str] = set() 
        self.warnings: List[str] = []
        self.blacklist_gav_keys = set(blacklist_gav_keys) if blacklist_gav_keys else set()

        # Disk-based POM cache setup
        self.pom_disk_cache_dir = pom_cache_dir
        if self.pom_disk_cache_dir:
            os.makedirs(self.pom_disk_cache_dir, exist_ok=True)

        self.check_artifact_existence = check_artifact_existence # NEW: Store the flag


    def _parse_version_string(self, version_str: str) -> List[int]:
        """
        Parses a version string into a list of integers for comparison.
        Handles common delimiters like '.', '-', and ignores text parts like '-jre', '-SNAPSHOT'.
        """
        if version_str is None:
            return []

        cleaned_version = version_str.lower()
        cleaned_version = re.sub(r'[-_.](alpha|beta|rc|m|final|ga|sp|jre|android|kmpc)(\d+)?', '', cleaned_version)
        cleaned_version = re.sub(r'-snapshot$', '', cleaned_version)

        parts = []
        for part in re.split(r'[^0-9]+', cleaned_version):
            if part.isdigit():
                parts.append(int(part))
        return parts

    def _get_version_parts(self, version_str: str) -> Tuple[int, int, int]:
        """
        Extracts major, minor, and patch components from a version string.
        Implements the following rules:
        1. Versions with no periods (e.g., "20240205") act like 0.0.X -> (0, 0, 20240205)
        2. Versions with one period (e.g., "1.2") act like A.B.X -> (1, 2, 0)
        3. Standard X.Y.Z versions -> (X, Y, Z)
        """
        parsed = self._parse_version_string(version_str)

        if not parsed:
            return (0, 0, 0)

        if len(parsed) == 1:
            return (0, 0, parsed[0])
        elif len(parsed) == 2:
            return (parsed[0], parsed[1], 0)
        else:
            return (parsed[0], parsed[1], parsed[2])

    def _are_versions_compatible(self, version1: str, version2: str) -> bool:
        """
        Checks if two versions are compatible based on semantic versioning rules,
        with a special rule for 0.x.x versions.
        Returns True if compatible, False otherwise.
        """
        if version1 is None or version2 is None:
            return True

        v1_major, v1_minor, v1_patch = self._get_version_parts(version1)
        v2_major, v2_minor, v2_patch = self._get_version_parts(version2)

        if v1_major != v2_major:
            if v1_major == 0 and v2_major == 0:
                if v1_minor != v2_minor:
                    return False
                return True
            return False

        return True

    def _is_version_higher(self, current_version: str, new_version: str) -> bool:
        """
        Compares two version strings to determine if new_version is strictly higher than current_version.
        Returns True if new_version is strictly higher.
        """
        if current_version is None:
            return True

        current_version_cleaned = self._clean_version_for_url(current_version)
        new_version_cleaned = self._clean_version_for_url(new_version)

        if not current_version_cleaned:
            return bool(new_version_cleaned)
        if not new_version_cleaned:
            return False

        current_parts = self._parse_version_string(current_version_cleaned)
        new_parts = self._parse_version_string(new_version_cleaned)

        max_len = max(len(current_parts), len(new_parts))
        for i in range(max_len):
            current_part = current_parts[i] if i < len(current_parts) else 0
            new_part = new_parts[i] if i < len(new_parts) else 0

            if new_part > current_part:
                return True
            if new_part < current_part:
                return False

        return len(new_parts) > len(current_parts) and new_parts[:len(current_parts)] == current_parts

    def _clean_version_for_url(self, version: str) -> str:
        """Removes common Maven version specifiers like brackets for URL construction."""
        if version is None:
            return None
        cleaned_version = version.replace('[', '').replace(']', '').replace('(', '').replace(')', '')
        if cleaned_version.upper() in ["LATEST", "RELEASE", "SNAPSHOT"]:
            return None
        return cleaned_version

    def _construct_base_artifact_url(self, base_repo_url: str, group_id: str, artifact_id: str, version: str) -> str:
        """
        Constructs the base URL path for an artifact (without extension).
        """
        cleaned_version = self._clean_version_for_url(version)
        if cleaned_version is None:
            raise ValueError(f"Cannot construct artifact URL for {group_id}:{artifact_id} with unresolvable version '{version}'.")

        group_path = group_id.replace('.', '/')
        return f"{base_repo_url}{group_path}/{artifact_id}/{cleaned_version}/{artifact_id}-{cleaned_version}"

    def get_artifact_download_url(self, group_id: str, artifact_id: str, version: str, packaging: str) -> Optional[str]:
        """
        Returns the direct download URL for a given artifact, trying configured repositories.
        Handles common packaging types that map to .jar files.
        It now checks for artifact existence and returns the URL only if found.
        """
        
        packaging_to_extension = {
            "jar": "jar",
            "bundle": "jar",
            "pom": "pom",
            "maven-plugin": "jar",
            "war": "war",
            "ear": "ear",
            "rar": "rar",
            "zip": "zip",
            "tar.gz": "tar.gz",
            "apk": "apk",
            "aar": "aar",
            "test-jar": "jar"
        }
        
        actual_extension = packaging_to_extension.get(packaging, "jar")

        for repo_url in self.repository_urls:
            try:
                base_url = self._construct_base_artifact_url(repo_url, group_id, artifact_id, version)
                download_url = f"{base_url}.{actual_extension}"
                
                # NEW LOGIC: Check if the artifact actually exists at this URL
                if self.check_artifact_existence: # Only check if the flag is enabled
                    if self._check_artifact_exists(download_url):
                        return download_url
                else:
                    return download_url # If not checking, return the first constructed URL
                
            except ValueError:
                # If URL construction fails for this repo, try next
                continue

        # If no artifact was found in any repository after checking (if enabled)
        warning_message = f"WARNING: Artifact file not found in any configured repository for {group_id}:{artifact_id}:{version} (packaging: {packaging})."
        print(warning_message, file=sys.stderr)
        self.warnings.append(warning_message)
        return None

    def _check_artifact_exists(self, url: str) -> bool:
        """
        Checks if an artifact exists at the given URL using a HEAD request.
        Returns True if the artifact exists (HTTP 200 OK), False otherwise.
        This function no longer adds warnings to self.warnings directly,
        as its primary purpose is to verify existence for other functions.
        """
        try:
            response = requests.head(url, timeout=5, allow_redirects=True)
            response.raise_for_status() # Raises HTTPError for bad responses (4xx or 5xx)
            return True
        except requests.exceptions.RequestException:
            # Do not add warning here; the calling function (get_artifact_download_url)
            # will handle the comprehensive warning if no repo yields the artifact.
            return False

    def _get_pom_disk_cache_path(self, group_id: str, artifact_id: str, version: str) -> str:
        """Constructs the local disk cache path for a POM file."""
        # Convert groupId to path format (e.g., com.google.guava -> com/google/guava)
        path_segments = group_id.replace('.', os.sep)
        return os.path.join(
            self.pom_disk_cache_dir,
            path_segments,
            artifact_id,
            version,
            f"{artifact_id}-{version}.pom"
        )

    def _fetch_pom(self, group_id: str, artifact_id: str, version: str) -> Optional[str]:
        """
        Fetches the POM content.
        Checks in-memory cache first, then disk cache, then remote repositories.
        Caches to disk if configured and downloaded from remote.
        """
        gav = f"{group_id}:{artifact_id}:{version}"
        
        # 1. Check in-memory cache
        if gav in self.pom_cache:
            return self.pom_cache[gav]

        pom_content = None
        disk_cache_path = None

        # 2. Check disk cache if configured
        if self.pom_disk_cache_dir:
            disk_cache_path = self._get_pom_disk_cache_path(group_id, artifact_id, version)
            if os.path.exists(disk_cache_path):
                try:
                    with open(disk_cache_path, 'r', encoding='utf-8') as f:
                        pom_content = f.read()
                    self.pom_cache[gav] = pom_content # Populate in-memory cache
                    print(f"INFO: Loaded POM from disk cache: {disk_cache_path}", file=sys.stderr)
                    return pom_content
                except Exception as e:
                    warning_message = f"WARNING: Could not read POM from disk cache {disk_cache_path}: {e}"
                    print(warning_message, file=sys.stderr)
                    self.warnings.append(warning_message)
                    # Continue to try remote if disk read fails

        # 3. If not in cache, fetch from remote repositories
        version_for_url = self._clean_version_for_url(version)
        if not version_for_url:
            return None

        for repo_url in self.repository_urls:
            try:
                pom_url = self._construct_base_artifact_url(repo_url, group_id, artifact_id, version) + ".pom"
            except ValueError:
                continue

            try:
                response = requests.get(pom_url, timeout=10)
                response.raise_for_status()
                pom_content = response.text
                self.pom_cache[gav] = pom_content # Populate in-memory cache

                # 4. Save to disk cache if configured
                if self.pom_disk_cache_dir and disk_cache_path:
                    os.makedirs(os.path.dirname(disk_cache_path), exist_ok=True)
                    with open(disk_cache_path, 'w', encoding='utf-8') as f:
                        f.write(pom_content)
                    print(f"INFO: Saved POM to disk cache: {disk_cache_path}", file=sys.stderr)
                
                print(f"INFO: Fetched POM from remote: {pom_url}", file=sys.stderr)
                return pom_content
            except requests.exceptions.RequestException:
                # Try next repository
                pass

        warning_message = f"WARNING: Failed to fetch POM for {gav} from any configured repository."
        print(warning_message, file=sys.stderr)
        self.warnings.append(warning_message)
        return None

    def _parse_pom_properties(self, root: ET.Element) -> Dict[str, str]:
        """Parses properties defined in a POM."""
        properties = {}
        properties_element = root.find('{http://maven.apache.org/POM/4.0.0}properties')
        if properties_element is not None:
            for prop in properties_element:
                tag_name = prop.tag.split('}')[-1] if '}' in prop.tag else prop.tag
                properties[tag_name] = prop.text
        return properties

    def _parse_project_packaging(self, root: ET.Element) -> Optional[str]:
        """Parses the <packaging> element from the project root."""
        packaging_element = root.find('{http://maven.apache.org/POM/4.0.0}packaging')
        if packaging_element is not None:
            return packaging_element.text
        return None

    def _resolve_property(self, value: str, properties: Dict[str, str]) -> str:
        """Resolves properties in a string, e.g., ${spring.version}."""
        if value is None:
            return None
        resolved_value = value
        for prop_name, prop_value in properties.items():
            placeholder = f"${{{prop_name}}}"
            resolved_value = resolved_value.replace(placeholder, prop_value if prop_value is not None else "")
        return resolved_value

    def _parse_dependencies(self, root: ET.Element, properties: Dict[str, str]) -> List[Dict[str, str]]:
        """Parses direct dependencies from a POM, applying properties and filtering optional ones."""
        dependencies = []
        dependencies_element = root.find('{http://maven.apache.org/POM/4.0.0}dependencies')
        if dependencies_element is not None:
            for dep_element in dependencies_element.findall('{http://maven.apache.org/POM/4.0.0}dependency'):
                group_id_element = dep_element.find('{http://maven.apache.org/POM/4.0.0}groupId')
                artifact_id_element = dep_element.find('{http://maven.apache.org/POM/4.0.0}artifactId')
                version_element = dep_element.find('{http://maven.apache.org/POM/4.0.0}version')
                scope_element = dep_element.find('{http://maven.apache.org/POM/4.0.0}scope')
                optional_element = dep_element.find('{http://maven.apache.org/POM/4.0.0}optional')

                if group_id_element is None or artifact_id_element is None:
                    continue

                group_id = self._resolve_property(group_id_element.text, properties)
                artifact_id = self._resolve_property(artifact_id_element.text, properties)
                
                dep_gav_key = f"{group_id}:{artifact_id}"
                if dep_gav_key in self.blacklist_gav_keys:
                    continue

                if self.ignore_optional_dependencies and optional_element is not None and optional_element.text == 'true':
                    continue

                version = self._resolve_property(version_element.text, properties) if version_element is not None else None
                scope = self._resolve_property(scope_element.text, properties) if scope_element is not None else "compile"

                if scope not in self.include_scopes:
                    continue

                if group_id and artifact_id and version:
                    dependencies.append({
                        "groupId": group_id,
                        "artifactId": artifact_id,
                        "version": version,
                        "scope": scope
                    })
        return dependencies

    def get_unique_transitive_dependencies(
        self,
        initial_dependencies_gav: List[str]
    ) -> List[Dict[str, str]]:
        """
        Performs a recursive transitive dependency analysis and returns a flat list
        of unique resolved artifacts (groupId:artifactId:version:packaging) for dependencies
        that match the desired scopes, applying "highest version wins" mediation and
        optionally ignoring optional dependencies. Tracks the first direct depender.
        Initial dependencies are always considered mandatory regardless of scope filtering.
        """
        queue: deque[Dict[str, str]] = deque()
        self.queued_gavs: Set[str] = set() 
        self.resolved_artifacts: Dict[str, Dict[str, str]] = {}
        self.pom_cache: Dict[str, str] = {} # Reset in-memory cache for each new analysis run
        self.warnings = []

        for dep_str in initial_dependencies_gav:
            parts = dep_str.split(':')
            if len(parts) == 3:
                group_id, artifact_id, version = parts
                scope = "compile"
            elif len(parts) == 4:
                if parts[3] in ["compile", "runtime", "provided", "test", "system", "import"]:
                    group_id, artifact_id, version = parts[0], parts[1], parts[2]
                    scope = parts[3]
                else:
                    group_id, artifact_id, _, version = parts
                    scope = "compile"
            else:
                warning_message = f"WARNING: Skipping malformed initial dependency: {dep_str}"
                print(warning_message, file=sys.stderr)
                self.warnings.append(warning_message)
                continue
            
            current_gav_key = f"{group_id}:{artifact_id}"
            current_candidate_full_gav = f"{current_gav_key}:{version}"

            if current_gav_key in self.blacklist_gav_keys:
                continue

            current_candidate_version = version 

            if current_gav_key in self.resolved_artifacts:
                existing_resolved_version = self.resolved_artifacts[current_gav_key]["version"]
                existing_depender = self.resolved_artifacts[current_gav_key]["first_depender"]
                
                if not self._are_versions_compatible(existing_resolved_version, current_candidate_version):
                    warning_message = (
                        f"WARNING: Incompatible version conflict for {current_gav_key}. "
                        f"Already resolved to {existing_resolved_version} (introduced by: {existing_depender}), "
                        f"new candidate {current_candidate_version} (introduced by: root project)."
                    )
                    print(warning_message, file=sys.stderr)
                    self.warnings.append(warning_message)

                if self._is_version_higher(existing_resolved_version, current_candidate_version):
                    self.resolved_artifacts[current_gav_key] = {
                        "version": current_candidate_version, 
                        "packaging": "jar",
                        "first_depender": "root project"
                    }
                    if current_candidate_full_gav not in self.queued_gavs:
                        queue.append({"groupId": group_id, "artifactId": artifact_id, "version": current_candidate_version, "scope": scope})
                        self.queued_gavs.add(current_candidate_full_gav)
            else:
                self.resolved_artifacts[current_gav_key] = {
                    "version": current_candidate_version, 
                    "packaging": "jar", 
                    "first_depender": "root project"
                }
                if current_candidate_full_gav not in self.queued_gavs:
                    queue.append({"groupId": group_id, "artifactId": artifact_id, "version": current_candidate_version, "scope": scope})
                    self.queued_gavs.add(current_candidate_full_gav)
            
        while queue:
            current_dependency_info = queue.popleft()
            group_id = current_dependency_info['groupId']
            artifact_id = current_dependency_info['artifactId']
            version_to_process = current_dependency_info['version']
            current_gav_full_in_queue = f"{group_id}:{artifact_id}:{version_to_process}"
            current_gav_key = f"{group_id}:{artifact_id}"

            winning_artifact_info = self.resolved_artifacts.get(current_gav_key)

            if not winning_artifact_info or winning_artifact_info["version"] != version_to_process:
                continue
            
            pom_content = self._fetch_pom(group_id, artifact_id, version_to_process)
            if not pom_content:
                continue
            
            try:
                root = ET.fromstring(pom_content)
                
                packaging_from_pom = self._parse_project_packaging(root)
                if packaging_from_pom:
                    if self.resolved_artifacts[current_gav_key]["version"] == version_to_process:
                        self.resolved_artifacts[current_gav_key]["packaging"] = packaging_from_pom

                parent_for_transitives_gav = current_gav_full_in_queue

                parent_properties = {}
                parent_element = root.find('{http://maven.apache.org/POM/4.0.0}parent')
                if parent_element is not None:
                    parent_group_id_elem = parent_element.find('{http://maven.apache.org/POM/4.0.0}groupId')
                    parent_artifact_id_elem = parent_element.find('{http://maven.apache.org/POM/4.0.0}artifactId')
                    parent_version_elem = parent_element.find('{http://maven.apache.org/POM/4.0.0}version')

                    if parent_group_id_elem is not None and parent_artifact_id_elem is not None and parent_version_elem is not None:
                        parent_group_id = self._resolve_property(parent_group_id_elem.text, {}) 
                        parent_artifact_id = self._resolve_property(parent_artifact_id_elem.text, {})
                        parent_version_candidate = self._resolve_property(parent_version_elem.text, {}) 

                        parent_gav_key = f"{parent_group_id}:{parent_artifact_id}"
                        parent_gav_full_candidate = f"{parent_gav_key}:{parent_version_candidate}"
                        
                        parent_depender_for_this_loop = parent_for_transitives_gav

                        if parent_gav_key in self.blacklist_gav_keys: 
                            parent_properties.update(self._parse_pom_properties(root)) 
                        elif parent_gav_key not in self.resolved_artifacts:
                            self.resolved_artifacts[parent_gav_key] = {
                                "version": parent_version_candidate,
                                "packaging": "pom", 
                                "first_depender": parent_depender_for_this_loop
                            }
                            if parent_gav_full_candidate not in self.queued_gavs:
                                queue.append({"groupId": parent_group_id, "artifactId": parent_artifact_id, "version": parent_version_candidate, "scope": "compile"}) 
                                self.queued_gavs.add(parent_gav_full_candidate)
                        else: 
                            existing_parent_version = self.resolved_artifacts[parent_gav_key]["version"]
                            existing_parent_depender = self.resolved_artifacts[parent_gav_key]["first_depender"]
                            
                            if not self._are_versions_compatible(existing_parent_version, parent_version_candidate):
                                warning_message = (
                                    f"WARNING: Incompatible version conflict for {parent_gav_key}. "
                                    f"Already resolved to {existing_parent_version} (introduced by: {existing_parent_depender}), "
                                    f"new candidate {parent_version_candidate} (introduced by: {parent_depender_for_this_loop})."
                                )
                                print(warning_message, file=sys.stderr)
                                self.warnings.append(warning_message)

                            if self._is_version_higher(existing_parent_version, parent_version_candidate):
                                self.resolved_artifacts[parent_gav_key].update({
                                    "version": parent_version_candidate,
                                    "packaging": "pom", 
                                    "first_depender": parent_depender_for_this_loop 
                                })
                                if parent_gav_full_candidate not in self.queued_gavs:
                                    queue.append({"groupId": parent_group_id, "artifactId": parent_artifact_id, "version": parent_version_candidate, "scope": "compile"})
                                    self.queued_gavs.add(parent_gav_full_candidate)

                        winning_parent_version = self.resolved_artifacts[parent_gav_key]["version"]
                        parent_pom_content_for_props = self._fetch_pom(
                            parent_group_id, parent_artifact_id, winning_parent_version
                        )
                        if parent_pom_content_for_props:
                            parent_root = ET.fromstring(parent_pom_content_for_props)
                            parent_properties = self._parse_pom_properties(parent_root)
                            parent_properties.update(self._parse_pom_properties(root)) 
                        else:
                            parent_properties = self._parse_pom_properties(root) 
                    else:
                        parent_properties = self._parse_pom_properties(root) 
                else:
                    parent_properties = self._parse_pom_properties(root) 

                direct_dependencies = self._parse_dependencies(root, parent_properties)

                for dep in direct_dependencies:
                    child_group_id = dep['groupId']
                    child_artifact_id = dep['artifactId']
                    child_version_candidate = dep['version']

                    child_gav_key = f"{child_group_id}:{child_artifact_id}"
                    child_gav_full_candidate = f"{child_gav_key}:{child_version_candidate}"

                    child_first_depender = parent_for_transitives_gav

                    if child_gav_key in self.resolved_artifacts:
                        existing_resolved_version = self.resolved_artifacts[child_gav_key]["version"]
                        existing_child_depender = self.resolved_artifacts[child_gav_key]["first_depender"]
                        
                        if not self._are_versions_compatible(existing_resolved_version, child_version_candidate):
                            warning_message = (
                                f"WARNING: Incompatible version conflict for {child_gav_key}. "
                                f"Already resolved to {existing_resolved_version} (introduced by: {existing_child_depender}), "
                                f"new candidate {child_version_candidate} (introduced by: {child_first_depender})."
                            )
                            print(warning_message, file=sys.stderr)
                            self.warnings.append(warning_message)

                        if self._is_version_higher(existing_resolved_version, child_version_candidate):
                            self.resolved_artifacts[child_gav_key] = {
                                "version": child_version_candidate, 
                                "packaging": "jar", 
                                "first_depender": child_first_depender
                            }
                            if child_gav_full_candidate not in self.queued_gavs:
                                queue.append(dep)
                                self.queued_gavs.add(child_gav_full_candidate)
                    else:
                        self.resolved_artifacts[child_gav_key] = {
                            "version": child_version_candidate, 
                            "packaging": "jar", 
                            "first_depender": child_first_depender
                        }
                        if child_gav_full_candidate not in self.queued_gavs:
                            queue.append(dep)
                            self.queued_gavs.add(child_gav_full_candidate)

            except ET.ParseError as e:
                warning_message = f"ERROR: Error parsing POM for {current_gav_key}:{version_to_process}: {e}"
                print(warning_message, file=sys.stderr)
                self.warnings.append(warning_message)
            except Exception as e:
                warning_message = f"ERROR: An unexpected error occurred for {current_gav_key}:{version_to_process}: {e}"
                print(warning_message, file=sys.stderr)
                self.warnings.append(warning_message)

        final_resolved_artifacts = []
        for ga_key, info in self.resolved_artifacts.items():
            group_id, artifact_id = ga_key.split(':')
            version = info["version"]
            packaging = info["packaging"]
            first_depender = info["first_depender"]
            
            # The get_artifact_download_url now handles existence checking across repos
            download_url = self.get_artifact_download_url(group_id, artifact_id, version, packaging)
            
            # artifact_exists is True if download_url is not None, False otherwise
            artifact_exists = download_url is not None

            final_resolved_artifacts.append({
                "groupId": group_id,
                "artifactId": artifact_id,
                "version": version,
                "packaging": packaging,
                "first_depender": first_depender,
                "download_url": download_url,
                "artifact_exists": artifact_exists
            })
        return final_resolved_artifacts

# --- Function to encapsulate analysis and return JSON-serializable data ---
def get_dependencies_as_json(
    initial_dependencies: List[str],
    repo_urls: List[str] = [
        "https://dl.google.com/dl/android/maven2/", # Prioritize direct Google Maven
        "https://maven.google.com/",
        "https://repo.maven.apache.org/maven2/" # Maven Central for general artifacts
    ],
    scopes_to_include: List[str] = ["compile", "runtime"], 
    ignore_optional: bool = True,
    exclude_runtime: bool = False,
    blacklist_gav_keys: Optional[List[str]] = None,
    pom_cache_dir: Optional[str] = None,
    check_artifact_existence: bool = False # NEW: Pass this argument
) -> Dict: 
    """
    Analyzes Maven dependencies and returns the unique resolved artifacts
    as a dictionary suitable for JSON serialization, including any incompatibility warnings.
    """
    analyzer = MavenDependencyAnalyzer(
        repository_urls=repo_urls,
        include_scopes=scopes_to_include, 
        ignore_optional_dependencies=ignore_optional,
        exclude_runtime_dependencies=exclude_runtime,
        blacklist_gav_keys=blacklist_gav_keys,
        pom_cache_dir=pom_cache_dir,
        check_artifact_existence=check_artifact_existence # NEW: Pass the flag
    )
    try:
        resolved_artifacts_info = analyzer.get_unique_transitive_dependencies(initial_dependencies)
        
        # Sort artifacts for consistent output
        sorted_artifacts = sorted(resolved_artifacts_info, key=lambda x: f"{x['groupId']}:{x['artifactId']}:{x['version']}")

        result = {
            "total_unique_dependencies": len(sorted_artifacts),
            "dependencies": sorted_artifacts,
            "warnings": analyzer.warnings 
        }
        return result

    except Exception as e:
        error_message = f"FATAL ERROR: An unhandled exception occurred during analysis: {e}"
        print(error_message, file=sys.stderr)
        return {"error": error_message, "status": "failed", "warnings": analyzer.warnings}

# --- Main execution block ---
if __name__ == "__main__":
    results_with_check = get_dependencies_as_json(
        initial_dependencies=[
            "androidx.activity:activity:1.9.3", # Original dependency
        ],
        pom_cache_dir=".pom_cache",
        check_artifact_existence=True, # Ensure artifact existence check is enabled
    )
    print(json.dumps(results_with_check, indent=2))
    print(f"Dependency count: {len(results_with_check['dependencies'])}")
    print("\n" + "="*80 + "\n")
    if results_with_check['warnings']:
        print("--- WARNINGS ---")
        for warning in results_with_check['warnings']:
            print(warning)
        print("----------------")
