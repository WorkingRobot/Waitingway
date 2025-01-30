#!/bin/bash

sudo systemctl start docker
docker-compose -f compose.dev.yaml up -d