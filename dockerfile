ARG REPO

#RUN apt-get install  curl gnupg lsb-release
#RUN mkdir -p /etc/apt/keyrings
#RUN curl -fsSL https://download.docker.com/linux/debian/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
#RUN echo \
      #"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian \
      #$(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
#RUN apt-get update && apt-get -y install libssl-dev docker-ce docker-ce-cli containerd.io docker-compose-plugin

FROM rust:latest

COPY ./src ./src
COPY ./templates ./templates
COPY ./Cargo.toml ./Cargo.toml

# set CROSS_CONTAINER_IN_CONTAINER to inform `cross` that it is executed from within a container
ENV CROSS_CONTAINER_IN_CONTAINER=true

RUN cargo install cross
RUN cargo build --release

CMD ./target/release/gload $REPO
expose 80
expose 3000
