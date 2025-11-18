#!/bin/bash

# Upload DARs to Canton Participant Node
# This script uploads all DAR files from the dars directory:
#   1. First uploads all dependency DARs from dars/dependencies/
#   2. Then uploads all CBTC DARs from dars/cbtc/
#
# Configuration:
#   - jwt_token: Authentication token for Canton API
#   - canton_admin_api_url: Canton participant node admin API URL

DAR_DIRECTORY="dars"
jwt_token=""

canton_admin_api_url="localhost:5002"
canton_admin_api_grpc_base_service="com.digitalasset.canton.admin.participant.v30"
canton_admin_api_grpc_package_service=${canton_admin_api_grpc_base_service}".PackageService"

json() {
  declare input=${1:-$(</dev/stdin)}
  printf '%s' "${input}" | jq -c .
}

upload_dar() {
  local dar_directory=$1
  local dar=$2
  echo "Uploading dar to ledger: ${dar}"

  # NOTE:
  # local base64_encoded_dar=$(base64 -w 0 ${dar_directory}/${dar})
  # The base64 command may require adopting to your unix environment.
  # The above example is based on the GNU base64 implementation.
  # The BSD version would look something like:
  local base64_encoded_dar=$(base64 -i ${dar_directory}/${dar} | tr -d '\n')

  local grpc_upload_dar_request="{
    \"dars\": [
      {
        \"bytes\": \"${base64_encoded_dar}\",
        \"description\": \"${dar}\"
      }
    ],
    \"vet_all_packages\": true,
    \"synchronize_vetting\": true
  }"

  # Only include Authorization header if jwt_token is set
  if [ -n "${jwt_token}" ]; then
    grpcurl \
      -plaintext \
      -H "Authorization: Bearer ${jwt_token}" \
      -d @ \
      ${canton_admin_api_url} ${canton_admin_api_grpc_package_service}.UploadDar \
      < <(echo ${grpc_upload_dar_request} | json)
  else
    grpcurl \
      -plaintext \
      -d @ \
      ${canton_admin_api_url} ${canton_admin_api_grpc_package_service}.UploadDar \
      < <(echo ${grpc_upload_dar_request} | json)
  fi

  echo "Dar '${dar}' successfully uploaded"
}

# Upload dependencies first (CBTC DARs depend on these)
DEPENDENCIES_DIR="${DAR_DIRECTORY}/dependencies"
if [ -d ${DEPENDENCIES_DIR} ]; then
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "Uploading dependency DARs..."
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  # List all .dar files in the dependencies directory
  dependency_dars=$(ls "${DEPENDENCIES_DIR}"/*.dar 2>/dev/null)

  # Loop over each dependency dar file
  for dar_path in ${dependency_dars}; do
    dar=$(basename ${dar_path})
    upload_dar ${DEPENDENCIES_DIR} ${dar}
  done

  echo "✓ All dependency DARs uploaded"
  echo ""
else
  echo "Dependencies directory not found: ${DEPENDENCIES_DIR}"
fi

# Upload CBTC DARs
CBTC_DIR="${DAR_DIRECTORY}/cbtc"
if [ -d ${CBTC_DIR} ]; then
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "Uploading CBTC DARs..."
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  # List all .dar files in the cbtc directory
  cbtc_dars=$(ls "${CBTC_DIR}"/*.dar 2>/dev/null)

  # Loop over each cbtc dar file
  for dar_path in ${cbtc_dars}; do
    dar=$(basename ${dar_path})
    upload_dar ${CBTC_DIR} ${dar}
  done

  echo "✓ All CBTC DARs uploaded"
  echo ""
else
  echo "CBTC directory not found: ${CBTC_DIR}"
fi

# Check if base directory exists
if [ ! -d ${DAR_DIRECTORY} ]; then
  echo "Base directory not found: ${DAR_DIRECTORY}"
  exit 1
fi
