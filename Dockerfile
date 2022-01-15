# syntax=docker/dockerfile:1
FROM debian:buster-slim as rtl_433_builder

RUN apt-get update \
    && apt-get install -y \
        build-essential \
        wget \
        git \
        cmake \
        pkg-config \
        libtool \
        libusb-1.0-0-dev \
        librtlsdr-dev \
        rtl-sdr \
    && rm -rf /var/lib/apt/lists/*

RUN cd /usr/local/src \
    && wget https://github.com/merbanan/rtl_433/archive/refs/tags/21.05.tar.gz \
    && tar xvf 21.05.tar.gz \
    && cd rtl_433-21.05 \
    && mkdir build \
    && cd build \
    && cmake .. \
    && make install \
    && cd ../.. && rm -rf rtl_433*

FROM rust:1.58.0-buster as weatherradio_builder

RUN apt-get update \
    && apt-get install -y \
        build-essential \
        wget \
        git \
        cmake \
        libssl-dev \
        libdbus-1-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/weatherradio

COPY . .

RUN cargo install --path .

FROM debian:buster-slim

RUN apt-get update \
    && apt-get install -y \
        libssl1.1 \
        libdbus-1-3 \
        rtl-sdr \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rtl_433_builder /usr/local/bin/rtl_433 /usr/local/bin/
COPY --from=weatherradio_builder /usr/local/cargo/bin/weatherradio /usr/local/bin/
RUN mkdir -p /root/.config/weatherradio

CMD ["weatherradio"]
