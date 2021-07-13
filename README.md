This is a simplified implementation of a restaurant api according to the requirements of https://github.com/paidy/interview/blob/master/SimpleRestaurantApi.md

# How to use:

**[Mac] or [Linux]**

open 3 shell sessions

#### session 1:
```shell
~$ git clone https://github.com/chenkai06ofc/restaurant-api.git
~$ cd restaurant-api
~/restaurant-api$ docker-compose up
```
Before ```docker-compose up```, make sure local port ```6379 & 3306``` are available, because the Redis & Mysql running in container will bind to local port.
(Sorry, I tried to put the everything into 1 docker-compose, but encountered some problems. )

#### session 2:
```shell
~$ cd restaurant-api
~/restaurant-api$ cargo build
~/restaurant-api$ SECS_PER_MIN=16 ./target/debug/restaurant-api
```
Naturally 1 minute=60 seconds, but for test purpose you can set 1 min to less secs to make time pass faster. (e.g ```SECS_PER_MIN=20``` will make 1 min=20 secs)

#### session 3:
```shell
~$ curl -X POST http://localhost:3000/item/add -d '{"table_no": 5, "content": "apple"}'
~$ curl -X POST http://localhost:3000/item/add -d '{"table_no": 5, "content": "rice"}'
~$ curl -X POST http://localhost:3000/item/add -d '{"table_no": 10, "content": "meat"}'
```

# API
### add item:
```shell
~$ curl -X POST http://localhost:3000/item/add -d '{"table_no": 5, "content": "apple"}'
```
### remove item:
```shell
~$ curl -X POST http://localhost:3000/item/remove -d '{"table_no": 5, "item_no": 2}'
```
### query specific table: 
(get item list of a specific table)
```shell
curl -X GET http://localhost:3000/item/query?table_no=5
```
### query specific item: 
(get the detail of a specific item)
```shell
curl -X GET http://localhost:3000/item/query?table_no=5&item_no=6
```

# Not Yet Implemented
### Message Queue
Let app server interact with MySQL is not so good, so I plan to add a MQ in between. Every RDB update request just send to MQ, another thread will poll and interact with MySQL.

