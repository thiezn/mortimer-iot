#!/bin/bash

# Script to pull and refresh pap.mortimer.nl

# Exit on error
set -e

# Variables
REPO_DIR="/root/pap.mortimer.nl"
DEPLOY_DIR="/var/www/pap.mortimer.nl"
SRC_DOC_DIR="$DEPLOY_DIR/src"
REPO_URL="git@github.com:thiezn/mortimeriot.git"

# Clone or pull latest changes. We first stash any possible local changes
echo "Pulling latest changes from repo"
if [ -d "$REPO_DIR" ]; then
    cd "$REPO_DIR"
    git add -A .
    git stash
    git pull origin main
else
    git clone "$REPO_URL" "$REPO_DIR"
    cd "$REPO_DIR"
fi

# Deploy site to web folder
echo "Replace web folder"
rm -rf "$DEPLOY_DIR"/*
cp -r * "$DEPLOY_DIR"
rm -rf "$DEPLOY_DIR"/.git

# Dump rust code base to llms.txt
echo "Create single llms.txt file"

LLM_FILE="$DEPLOY_DIR/llms.txt"


echo "pap.mortimer.nl website!! " > $LLM_FILE

# Set ownership
chown -R www-data:www-data "$DEPLOY_DIR"

# Restart Apache
echo "Restarting Apache"
/usr/sbin/service apache2 restart

# Notify Bing about the new sitemap (Google ping is deprecated)
# echo "Notifying Bing about sitemap update..."
# curl -s "https://www.bing.com/ping?sitemap=$SITE_ROOT/sitemap.xml" || echo "Bing ping failed"

