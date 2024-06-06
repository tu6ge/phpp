FROM mcr.microsoft.com/devcontainers/php:1-8.2-bullseye

RUN docker-php-ext-install mysqli
RUN docker-php-ext-install bcmath
#RUN docker-php-ext-install json
RUN docker-php-ext-install curl
# RUN docker-php-ext-install hash
RUN curl -sSLf \
        -o /usr/local/bin/install-php-extensions \
        https://github.com/mlocati/docker-php-extension-installer/releases/latest/download/install-php-extensions && \
    chmod +x /usr/local/bin/install-php-extensions

RUN install-php-extensions mbstring
RUN install-php-extensions gmp
RUN install-php-extensions ftp
RUN install-php-extensions mongodb
RUN install-php-extensions zip