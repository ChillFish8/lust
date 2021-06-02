FROM nginx:latest

# Remove the default Nginx configuration file
RUN rm -v /etc/nginx/nginx.conf

# Copy a configuration file from the current directory
ADD nginx.conf /etc/nginx/

ADD sample /usr/share/nginx/html/
ADD sample /var/www/html/

# Expose ports
EXPOSE 90
