#!/bin/sh

diesel migration redo

rm -rf ./.stage/entities/*

sea-orm-cli generate entity -o ./.stage/entities \
    --model-extra-derives 'utoipa::ToSchema,serde::Serialize,serde::Deserialize' \
    --enum-extra-derives 'utoipa::ToSchema,serde::Serialize,serde::Deserialize'
