import Kafka from 'node-rdkafka';

const topic = 'settopbox'
const categories = ['CHANNEL', 'STATE', 'SOURCE']

console.log("NOTICE: Connecting to kafka broker at ", process.env.KAFKA_HOST)
const kafkaConfig = {
  'bootstrap.servers': process.env.KAFKA_HOST,
  'security.protocol': 'SASL_PLAINTEXT',
  'request.timeout.ms': 30000,
  'sasl.mechanisms': 'PLAIN',
  'sasl.mechanism': 'PLAIN',
  'sasl.username': process.env.KAFKA_USERNAME,
  'sasl.password': process.env.KAFKA_PASSWORD,
  'session.timeout.ms': 45000
}

const adminClient = Kafka.AdminClient.create(kafkaConfig)
adminClient.createTopic({
      topic: topic,
      num_partitions: 1,
      replication_factor: 3
    }, function(err) {
      console.log(`response::`+err);
    });

const stream = Kafka.Producer.createWriteStream(
  kafkaConfig, {}, {
  topic: topic
});

stream.on('error', (err) => {
  console.error('Error in our kafka stream');
  console.error(err);
});

function queueRandomMessage() {
  const category = getRandomCategory();
  const status = getRandomData(category);
  const event = { category, status };
  event.timestamp = Date.now()
  // const success = stream.write(eventType.toBuffer(event));     
  const success = stream.write(Buffer.from(JSON.stringify(event)));     
  if (success) {
    console.log(`message queued (${JSON.stringify(event)})`);
  } else {
    console.log('Too many messages in the queue already..');
  }
}

function getRandomCategory() {
  return categories[Math.floor(Math.random() * categories.length)];
}

function getRandomData(category) {
  let statuses
  if (category === 'CHANNEL') {
    statuses = ['NBC', 'CNN', 'CBS', 'FOX'];
  } else if (category === 'STATE') {
    statuses = ['PLAY', 'OFF', 'PAUSE'];
  } else if (category === 'SOURCE') {
    statuses = ['DTS', 'HDMI1','SMARTTV', 'HDMI2', 'AV1'];
  } else {
    statuses = ['UNKNOWN'];
  }
  return statuses[Math.floor(Math.random() * statuses.length)];
}

setInterval(() => {
  queueRandomMessage();
}, 3000);