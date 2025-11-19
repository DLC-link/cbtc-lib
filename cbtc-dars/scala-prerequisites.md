# Canton scripts

## Prerequisite

- jq
- curl
- java\*
- canton 3.3.0

\*To run it natively you need Java 11 or higher installed on your system.

### Canton 3.3.0

_Canton installation helps to communicate with the canton node._

Run `canton --version` and verify you are running 3.3.0. The output should look similar:

```bash
Canton: 3.3.0-SNAPSHOT
Daml Libraries: 3.3.0-snapshot.20250502.13767.0.v2fc6c7e2
Stable Canton protocol versions: List(33)
Preview Canton protocol versions: List()
```

If you don't have this available on your workstation, you can download the 3.3 canton.jar [here](https://github.com/digital-asset/daml/releases/download/v3.3.0-snapshot.20250603.0/canton-open-source-3.3.0-snapshot.20250530.15919.0.v3e7a341c.tar.gz), unzip this, find either the executable under the 'bin' directory, or the jar under the 'lib' directory and build an alias to run the jar as a canton command.

#### Install Canton command in terminal

1. Download the compressed file

```bash
curl -L -C - \
  -o canton-open-source-3.3.0-snapshot.20250530.15919.0.v3e7a341c.tar.gz \
  "https://github.com/digital-asset/daml/releases/download/v3.3.0-snapshot.20250603.0/canton-open-source-3.3.0-snapshot.20250530.15919.0.v3e7a341c.tar.gz"

# OR

wget https://github.com/digital-asset/daml/releases/download/v3.3.0-snapshot.20250603.0/canton-open-source-3.3.0-snapshot.20250530.15919.0.v3e7a341c.tar.gz
```

2. Uncompress the file

```bash
tar -xvzf canton-open-source-3.3.0-snapshot.20250530.15919.0.v3e7a341c.tar.gz -C /opt
```

3. Make an alias so it can be called much easier

```bash
alias canton='java -jar /opt/canton-open-source-3.3.0-snapshot.20250530.15919.0.v3e7a341c/lib/canton-open-source-3.3.0-snapshot.20250530.15919.0.v3e7a341c.jar'
```
