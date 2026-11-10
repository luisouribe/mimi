#!/bin/sh
# Creates a personal certificate for signing Mimi on this Mac, in your login keychain.
# With it, macOS keeps Mimi's Accessibility permission from one build to the next.
# It's valid only for your own builds, for 10 years. The private key never leaves this Mac.
# Run it once, yourself: sh tools/mac/create_signing_certificate.sh
set -eu

name="Mimi Local Signing"
keychain="$HOME/Library/Keychains/login.keychain-db"

if security find-certificate -c "$name" "$keychain" >/dev/null 2>&1; then
  echo "The certificate \"$name\" already exists. Nothing to do."
  exit 0
fi

umask 077
work=$(mktemp -d)
trap 'rm -rf "${work:?}"' EXIT

cat > "$work/request.cnf" <<EOF
[req]
distinguished_name = subject
x509_extensions = signing
prompt = no

[subject]
CN = $name

[signing]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
EOF

/usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -sha256 \
  -config "$work/request.cnf" -keyout "$work/key.pem" -out "$work/certificate.pem" 2>/dev/null

# Only used for this moment, to carry the key into the keychain.
transfer=$(/usr/bin/openssl rand -hex 24)
/usr/bin/openssl pkcs12 -export -inkey "$work/key.pem" -in "$work/certificate.pem" \
  -name "$name" -out "$work/mimi.p12" -passout "pass:$transfer"

security import "$work/mimi.p12" -k "$keychain" -P "$transfer" -T /usr/bin/codesign >/dev/null

echo "Done: \"$name\" is in your login keychain."
