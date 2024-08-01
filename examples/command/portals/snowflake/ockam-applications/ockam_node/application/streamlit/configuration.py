from dataclasses import dataclass
import streamlit as st
import snowflake.permissions as permission
from snowflake.snowpark.functions import call_udf, lit
from snowflake.snowpark import Session
import json
import yaml

# Get the current credentials
session = Session.builder.getOrCreate()

# This flag can be used to test the application with preset configuration values
TEST = True


# Main workflow for the configuration application:
#  - The user must input the Ockam project URL + the configuration file including the enrollment ticket
#  - Then there is a link to grant the permission to create a network rule to access the project
#  - Finally the application can be started
def setup():
    if "configured" not in st.session_state:
        st.header("Ockam node configuration")
        st.write("""
           This application lets you create an Ockam node which allows you to communicate with a private network. 
        """)
        st.info("A full example showing how to use this Ockam node to connect to a private Postgres database is documented [here](https://github.com/build-trust/ockam/blob/develop/examples/command/portals/snowflake/example-6/README.md).")
        st.subheader("Prerequisites")
        st.write("""
                You first need to [sign-up for Ockam](https://www.ockam.io/download), then download `ockam` and enroll to create
                your first project:
                ```
                curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
                source "$HOME/.ockam/env"

                ockam enroll
                ```
                
                Then you need to provide the Ockam project URL and configuration file.
            """)

        st.subheader("Ockam project URL")
        st.write("""
            The Ockam project URL can be retrieved by getting the value returned by the following command:
            ```
            ockam project show --jq .egress_allow_list
            ```
            """)
        if TEST:
            url = st.text_input("Ockam project URL",
                                value="k8s-hub-nginxing-a610bd423b-c1518c29eb96c4c1.elb.us-west-1.amazonaws.com:4004",
                                label_visibility="collapsed")
        else:
            url = st.text_input("Ockam project URL",
                                placeholder="k8s-hub-nginxing-a610bd423b-c1518c29eb96c4c1.elb.us-west-1.amazonaws.com:4004",
                                label_visibility="collapsed")

        st.subheader("Configuration")
        st.write("""
            Configuration file for the Ockam node. That file must contain a `ticket` field with an enrollment ticket.
            
            Please consult the [`ockam node create` command help](https://command.ockam.io/manual/ockam-node-create.html) 
            for more information. 
            """)
        configuration_example = """node: ockam-inlet
tcp-listener-address: 0.0.0.0:0
ticket: 7b226f6e655f74696d655f636f6465223a2239366538303635646464313463313932666234333065356639326139646539613263343838396562313163393366363562346662636335623566636230613634222c2270726f6a656374223a7b226964223a2238313935626434332d656234662d346534612d393635302d386665643163633032386135222c226e616d65223a2264656661756c74222c2273706163655f6e616d65223a227375727072697365642d6e696c676169222c226163636573735f726f757465223a222f646e73616464722f6b38732d6875622d6e67696e78696e672d613631306264343233622d633135313863323965623936633463312e656c622e75732d776573742d312e616d617a6f6e6177732e636f6d2f7463702f343030342f736572766963652f617069222c227573657273223a5b5d2c2273706163655f6964223a2231633863653239312d386164332d343533352d393365392d383130616131383839346262222c226964656e74697479223a224939666163663430353131326465656338363564393534323965323661353165663132306233366334336239623331306231333836383665653237306638383230222c22617574686f726974795f6163636573735f726f757465223a222f646e73616464722f6b38732d6875622d6e67696e78696e672d613631306264343233622d633135313863323965623936633463312e656c622e75732d776573742d312e616d617a6f6e6177732e636f6d2f7463702f343030342f736572766963652f617574686f726974792f736572766963652f617069222c22617574686f726974795f6964656e74697479223a2238313832353833373833303130313538333238356636383230303831353832303535386466343235343331633263313138313732363065313461656133333532376237363138613933323565333435643538356161343866343931663862323466343161363661303964303531613739366361303035383230303831353834303433623263333264306539326135356435376238376566613835333038626564386331653261653832313131323938633937626135333931633632656130313464366436353566343863343962306339383962393532643933663334333934396665626165636163333761363436343937346631623033323961633765653030222c2276657273696f6e223a22302e372e31222c2272756e6e696e67223a747275652c226f7065726174696f6e5f6964223a6e756c6c2c22757365725f726f6c6573223a5b7b22656d61696c223a2265746f727265626f727265407961686f6f2e636f6d222c226964223a3231352c22726f6c65223a2241646d696e222c2273636f7065223a225370616365227d5d2c2270726f6a6563745f6368616e67655f686973746f7279223a2238313832353833373833303130313538333238356636383230303831353832303736653564303437376164316566626262336639643061613538316462326535333534313563306533643732313330663239383164333462353139346636613766343161363661303964303331613739366361303033383230303831353834303339613136633434306134633232633764373062616363613365383839373430356637633936373563616531346535653334666263373337356231363536353834383264343661366361656234356233663138383233613864323562383034623061323934343963326466623437306165636264343261633166356230303032227d7d 
tcp-inlet: 
  from: 0.0.0.0:5433 
  via: postgres
  allow: postgres_server"""

        height = 600
        if TEST:
            configuration = st.text_area("Configuration", value=configuration_example,
                                         label_visibility="collapsed",
                                         height=height)
        else:
            configuration = st.text_area("Configuration", placeholder=configuration_example,
                                         label_visibility="collapsed",
                                         height=height)

        if st.button("Submit"):
            st.session_state.configured = {
                "node_type": "general",
                "configuration": configuration,
            }
            session.sql(f"DELETE FROM internal.ockam_project_url").collect()
            session.sql(f"INSERT INTO internal.ockam_project_url VALUES ('{url}')").collect()
            st.rerun()

    else:
        # Grant access to the Ockam project URL if that has not been done yet
        for ref in get_references():
            name = ref.name
            if not ref.bound_alias:
                st.button(f"Grant access to your Ockam project ↗", on_click=permission.request_reference, args=[name],
                          key=name)
            else:
                st.caption(f"Your Ockam project is accessible ✅")
            if not ref.bound_alias: return
        start_the_application()


# Start the application using the configuration data provided by the user
def start_the_application():
    if st.button("Start the application", type="primary"):
        configured = st.session_state.configured
        port = read_port(configured["configuration"])
        st.caption(f"Starting the Ockam node at ockam-endpoint:{port}...")

        # This conversion avoids quoting and newline issues when the configuration is passed as
        # an argument to the ockam executable
        configuration_as_one_liner = yaml.dump(yaml.safe_load(configured["configuration"]),
                                               default_flow_style=True)
        result = session.call('external.start_ockam_node_service', configuration_as_one_liner, port)

        if result == 'SUCCESS':
            st.session_state.configuration_done = True
            st.session_state.port = port
            st.rerun()
        else:
            st.error(f"The Ockam service could not be started: {result}.\nPlease consult the logs", icon="🚨")


# Welcome page displayed when everything has been properly configured
def welcome():
    st.title('Ockam node')
    st.write(
        f"""
        Your Ockam node is now running and available at `ockam-endpoint:{st.session_state.port}` 🚀
        """)


# Application reference
@dataclass
class Reference:
    name: str
    label: str
    type: str
    description: str
    bound_alias: str


# Return the list of application references associated with the application
# There should only be one which is of type CONSUMER_EXTERNAL_INTEGRATION
def get_references():
    app_name = session.get_current_database()
    data_frame = session.create_dataframe([''])
    refs = data_frame.select(call_udf('system$get_reference_definitions', lit(app_name))).collect()[0][0]
    references = []
    for row in json.loads(refs):
        bound_alias = row["bindings"][0]["alias"] if row["bindings"] else None
        references.append(Reference(row["name"], row["label"], row["object_type"], row["description"], bound_alias))
    return references


def read_port(configuration_as_string):
    configuration = yaml.safe_load(configuration_as_string)
    if "tcp-inlet" in configuration:
        if "from" in configuration["tcp-inlet"]:
            host = configuration["tcp-inlet"]["from"]
            port = host.split(":")[-1]
            return port
    if "tcp-outlet" in configuration:
        if "to" in configuration["tcp-outlet"]:
            host = configuration["tcp-outlet"]["to"]
            port = host.split(":")[-1]
            return port
    return "5432"


if __name__ == '__main__':
    if 'configuration_done' not in st.session_state:
        setup()
    else:
        welcome()
