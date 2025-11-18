# CBTC DAR releases

User facing DAR packages. These must be uploaded to your participant node where you intend to Mint/Burn CBTC.

## Upload scripts

There are 2 main ways to upload DARs to your participant node:

- RECOMMENDED: gRPC - using the [upload_dars.sh](upload_dars.sh) script.
- more involved: Using canton binary with the [UploadDars.sc](UploadDars.sc) script. For that check the [Scala prerequisites](scala-prerequisites.md) doc.

For both methods you will need to have port-forwarding to your participant's admin API port.
