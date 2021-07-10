FROM ubuntu:latest
WORKDIR /
COPY ["./target/debug/restaurant-api", "."]
RUN chmod a+x ./restaurant-api
EXPOSE 3000
CMD [ "./restaurant-api" ]