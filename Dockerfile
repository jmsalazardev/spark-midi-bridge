ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

RUN dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install --assume-yes libasound2-dev:armhf
