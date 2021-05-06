# defmodule Kafka.Publish do
#   def publish(topic, message) do
#     KafkaEx.create_worker(:publish)
#     KafkaEx.produce(topic, 0, message, worker_name: :publish)
#   end
# end

## Kafka.Publish.publish("consumer_only_stream_example", "message")
