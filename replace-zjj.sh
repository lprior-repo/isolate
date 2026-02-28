#!/bin/bash
# Replace all Isolate/isolate with Isolate/isolate (case-sensitive) throughout the codebase

find . -type f ! -path "./.git/*" \
	-exec sed -i 's/Isolate/Isolate/g; s/isolate/isolate/g' {} +

echo "Done. Replaced Isolate→Isolate and isolate→isolate in all files (except .git)."
