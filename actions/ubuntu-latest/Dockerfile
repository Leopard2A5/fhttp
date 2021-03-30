FROM ubuntu:latest

# for automatic installation of tzdata
ENV DEBIAN_FRONTEND=noninteractive

COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
