# AMQP example

```
sudo rabbitmqctl add_vhost webwire
sudo rabbitmqctl add_user webwire webwire
sudo rabbitmqctl set_permissions --vhost webwire webwire ".*" ".*" ".*"
```
