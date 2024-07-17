# Change Data Capture Application

This application captures changes from Snowflake tables and sends them to a private Kafka instance.
The data is sent encrypted, with mutual authentication, over a secure channel managed via [Ockam](http://ockam.io).
