#!/bin/bash

order_id="b563feb7b2b84b6test"

if [ ! -z $1 ];
then
    order_id=$1
fi

url="http://localhost:3001/orders/$order_id/payment"

echo "$(curl -v "$url" -H "Content-Type: application/json")"
