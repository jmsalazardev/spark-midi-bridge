ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

RUN rm -f /etc/apt/preferences.d/* && \
    dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install --assume-yes libasound2-dev:armhf
