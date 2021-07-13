# restaurant-api

This is a simplified implementation of a restaurant api according to the requirements of https://github.com/paidy/interview/blob/master/SimpleRestaurantApi.md

## How to use:

**[Mac] or [Linux]**

open 3 shell sessions

##### session 1:
```shell
~$ git clone https://github.com/chenkai06ofc/restaurant-api.git
~$ cd restaurant-api
~/restaurant-api$ docker-compose up
```

##### session 2:
```shell
~$ cd restaurant-api
~/restaurant-api$ cargo build
~/restaurant-api$ SECS_PER_MIN=16 ./target/debug/restaurant-api
```

##### session 3:
```shell
~$ curl -X POST http://localhost:3000/item/add -d '{"table_no": 5, "content": "apple"}'
~$ curl -X POST http://localhost:3000/item/add -d '{"table_no": 5, "content": "rice"}'
~$ curl -X POST http://localhost:3000/item/add -d '{"table_no": 10, "content": "meat"}'
```

## API
##### add item:
```shell
~$ curl -X POST http://localhost:3000/item/add -d '{"table_no": 5, "content": "apple"}'
```
##### remove item:
```shell
~$ curl -X POST http://localhost:3000/item/remove -d '{"table_no": 5, "item_no": 2}'
```
##### query specific table:
```shell
curl -X GET http://localhost:3000/item/query?table_no=5
```
##### query specific item:
```shell
curl -X GET http://localhost:3000/item/query?table_no=5&item_no=6
```
