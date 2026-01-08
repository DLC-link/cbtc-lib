# CBTC DAR releases

User facing DAR packages. These must be uploaded to your participant node where you intend to Mint/Burn CBTC.

Note: these packages are only needed for CBTC minting / burning (redemption) and not for other CBTC transactions such as sending and receiving, or UTXO management.

## Upload scripts

There are 2 main ways to upload DARs to your participant node:

- **RECOMMENDED**: gRPC - using the [upload_dars.sh](upload_dars.sh) script.
- more involved: Using canton binary with the [UploadDars.sc](00_UploadDars.sc) script. For that check the [Scala prerequisites](scala-prerequisites.md) doc.

For both methods you will need to have port-forwarding to your participant's admin API port.

## Using the gRPC upload script

### Prerequisites

- Port-forward to your participant admin API port (default: 5002)
- Get a JWT token for your participant admin (Optional: If your participant's admin port does not require authentication, you can skip this step)

### Configuration

1. **Set your JWT token** (if authentication is required):

   ```bash
   # Option 1: Export as environment variable
   export jwt_token="your-jwt-token-here"

   # Option 2: Edit the script directly
   # Open upload_dars.sh and set: jwt_token="your-jwt-token-here"
   ```

2. **Configure Canton admin API URL** (optional):
   - Default is `localhost:5002`
   - If your participant uses a different host/port, edit `canton_admin_api_url` in the script

### Run the upload script

```bash
./upload_dars.sh
```

The script will automatically:

1. Upload all dependency DARs from `dars/dependencies/` (these are uploaded first since CBTC DARs depend on them)
2. Upload all CBTC DARs from `dars/cbtc/`

**Note:** If your node already has a given DAR, Canton will automatically skip it. You can safely re-run the script without worrying about duplicate uploads.

## Using the Scala scripts

### Prerequisite

Please check whether you have everything set up properly [here](scala-prerequisites.md).

### Run the Scala script

1. Run the Scala script to upload dars

```bash
canton run 00_UploadDars.sc -c ./misc/connect.conf
```

Note that if your node already has a given DAR, it will automatically skip the upload for that DAR. So you can safely re-run the script if needed, and not have to worry about duplicate uploads.

2. Validate CBTC DARs

```bash
canton run 00_ValidateDars.sc -c ./misc/connect.conf
```
