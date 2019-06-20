# Minimum Criteria for Trust

We need a set of explicit criteria to decide if we can trust any new message enough to act on it. Trust within connected systems is context specific, multi-faceted and nuanced. Solution builders must decide which criteria build trust within a given context.

Different scenarios may require varying degrees of trust. For instance, a firmware update may require a greater degree of trust than a message from a temperature sensor in a weather station. In contrast, a temperature sensor message from a machine on a factory floor that could cause an emergency shutdown, if it crosses a threshold, should require a really high degree of trust. Ockam’s goal is to enable system builders to easily gauge against such criteria and build secure systems that can be relied on.

Security and privacy are great guiding principles for a minimum criteria that must be thoughtfully considered in every message exchange. The STRIDE model, developed in 1999 by Praerit Garg and Loren Kohnfelder at Microsoft, provides an easy to remember mnemonic for the minimum set of threats that every messaging based system must consider:

|       | Threat	               | Desired property  |
|-------|------------------------|-------------------|
| **S** | Spoofing identity      | Authenticity      |
| **T** | Tampering with data	   | Integrity         |
| **R** | Repudiation	           | Non-repudiability |
| **I** | Information disclosure | Confidentiality   |
| **D** | Denial of service	     | Availability      |
| **E** | Elevation of privilege | Authorization     |


In a situation where entity B receives a message, The STRIDE model gives B a minimum starting criteria to gauge if it should trust this message:

<img width="900" alt="Message from A to B" src="message-from-a-to-b.png">

__Identification__
* Figure out who the message is from. (answer in the above case may be A).

__Authenticity__
* Prove to a reasonable level of assurance that the message really came from A and the identification process was not fooled.

__Integrity__
* Prove to a reasonable level of assurance that the message B has received is exactly the message A sent.
* Prove to a reasonable level of assurance that the message B has received is not a replay of a message A created in the past.

__Non-repudiability__
* Prove to a reasonable level of assurance that given what B knows, B could prove to someone else that the message came from A.

__Confidentiality__
* Prove that someone other that A or B did not see the contents of message while in transit. (Encryption)
* Prove that a future compromise of B's private keys will not expose past session data. (Perfect Forward Secrecy)
* Prove that someone other that A or B was not able to observe metadata about which entities exchanged messages and when. (Privacy)
* Prove that A would not be able to co-relatable B's activity with other publicly available activity. (Privacy)
* Prove that private key material is not accidentally leaked during the exchange.

__Availability__
* Prove that the message is not a malicious attempt to overwhelm B and take it out of service.

__Authorization__
* Prove that A is allowed to produce information that is contained in the message.
* Prove that the message is not a malicious attempt to escalate privilege.

While the above list of rules is not exhaustive, it provides a robust framework to start reasoning about trust within systems, one message exchange at a time.

Application needs will vary, some systems will prefer to have repudiability to favor privacy, while other applications would prefer to have non-repudiability to favor accountability, some scenarios would demand confidentiality while others won't.

However, __all connected systems must have at least a guarantee of data integrity__. Any system without trust in integrity of messages cannot be relied on and is rendered futile. It is also important to note that __proving data integrity requires identification and authenticity__. Hence the __absolute minimum criteria for trust is Identification, Authenticity and Integrity__.

Achieving all of the above criteria needs careful and robust implementations of identity management, credential management, cryptography, authentication and messaging protocols. Ockam SDK abstracts this complicity and makes it easy for you to develop reliable and trustworthy systems.
