FROM ghcr.io/rust-cross/cargo-zigbuild:latest AS builder
ARG VERSION
ARG TARGET
ARG ARCH

ENV VERSION=${VERSION}
ENV TARGET=${TARGET}
ENV ARCH=${ARCH}
ENV PKG_CONFIG_ALL_STATIC=1
ENV PKG_CONFIG_SYSROOT_DIR=/
ENV ac_cv_func_malloc_0_nonnull=yes

SHELL ["/bin/bash", "-c"]

RUN apt-get update && apt-get install -y \
    build-essential \
    wget \
    pkg-config \
    ca-certificates \
    flex \
    bison \
 && rm -rf /var/lib/apt/lists/*


WORKDIR /build


COPY Cargo.toml Cargo.toml

RUN mkdir src && echo "fn main(){}" > src/main.rs


RUN rustup target add ${TARGET}
RUN cargo zigbuild --release --target ${TARGET} || true


COPY ./src ./src
RUN cargo zigbuild --release --target ${TARGET}

RUN mkdir /out && \
    cp target/${TARGET}/release/dxon \
       /out/dxon-${VERSION}-linux-${ARCH}

FROM scratch
COPY --from=builder /out /