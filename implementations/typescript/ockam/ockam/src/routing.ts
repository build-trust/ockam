
/**
 * An 8-bit unsigned integer.
 */
type uint8 =
  0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
  16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 25 | 26 | 27 | 28 | 29 | 30 | 31 |
  32 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40 | 41 | 42 | 43 | 44 | 45 | 46 | 47 |
  48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 | 58 | 59 | 60 | 61 | 62 | 63 |
  64 | 65 | 66 | 67 | 68 | 69 | 70 | 71 | 72 | 73 | 74 | 75 | 76 | 77 | 78 | 79 |
  80 | 81 | 82 | 83 | 84 | 85 | 86 | 87 | 88 | 89 | 90 | 91 | 92 | 93 | 94 | 95 |
  96 | 97 | 98 | 99 | 100 | 101 | 102 | 103 | 104 | 105 | 106 | 107 | 108 | 109 | 110 | 111 |
  112 | 113 | 114 | 115 | 116 | 117 | 118 | 119 | 120 | 121 | 122 | 123 | 124 | 125 | 126 | 127 |
  128 | 129 | 130 | 131 | 132 | 133 | 134 | 135 | 136 | 137 | 138 | 139 | 140 | 141 | 142 | 143 |
  144 | 145 | 146 | 147 | 148 | 149 | 150 | 151 | 152 | 153 | 154 | 155 | 156 | 157 | 158 | 159 |
  160 | 161 | 162 | 163 | 164 | 165 | 166 | 167 | 168 | 169 | 170 | 171 | 172 | 173 | 174 | 175 |
  176 | 177 | 178 | 179 | 180 | 181 | 182 | 183 | 184 | 185 | 186 | 187 | 188 | 189 | 190 | 191 |
  192 | 193 | 194 | 195 | 196 | 197 | 198 | 199 | 200 | 201 | 202 | 203 | 204 | 205 | 206 | 207 |
  208 | 209 | 210 | 211 | 212 | 213 | 214 | 215 | 216 | 217 | 218 | 219 | 220 | 221 | 222 | 223 |
  224 | 225 | 226 | 227 | 228 | 229 | 230 | 231 | 232 | 233 | 234 | 235 | 236 | 237 | 238 | 239 |
  240 | 241 | 242 | 243 | 244 | 245 | 246 | 247 | 248 | 249 | 250 | 251 | 252 | 253 | 254 | 255;

// type uint8 = 0..255;
// This would be nice, but it's not supported by Typescript yet.
// https://github.com/Microsoft/TypeScript/issues/15480

/**
 * The type of a local address.
 */
export const LOCAL = 0;

/**
 * The type of a tcp address.
 */
export const TCP = 1;

/**
 * The type of a udp address.
 */
export const UDP = 2;

/**
 * The type of a websocket address.
 */
export const WS = 3;

/**
 * The type of a routing address.
 */
export type AddressType = uint8;

/**
 * A routing address.
 */
export type Address = string | Uint8Array | [type: AddressType, value: any];

/**
 * A routing route.
 */
export type Route = Array<Address>;

/**
 * A routable message.
 */
export interface Message { onwardRoute: Route, returnRoute: Route, payload: any };

/**
 * A message handler
 */
export type MessageHandler = (message: Message) => void;

/**
 * A router plugin.
 */
export interface AddressTypePlugin {
  messageHandler: MessageHandler
  convertAddressToString: (address: Address) => string | undefined
  convertAddressToUint8Array: (address: Address) => Uint8Array | undefined
}

/**
 * A router that route a message to a registered handler based on the.
 */
export class Router {
  plugins: Partial<Record<AddressType, AddressTypePlugin>>;
  unroutableMessageHandler: MessageHandler;

  constructor(unroutableMessageHandler: MessageHandler) {
    this.unroutableMessageHandler = unroutableMessageHandler;
    this.plugins = {};
  }

  route(message: Message) {
    this.getHandler(message.onwardRoute[0])(message);
  }

  getAddressType(address: Address) {
    if (Array.isArray(address)) {
      if (address.length == 2) {
        let first = address[0];
        if ((typeof first === "number") && first >= 0 && first <= 255) {
          return address[0]
        }
      }
    }

    return 0;
  }

  getHandler(address: undefined | Address) {
    if (address) {
      let plugin = this.plugins[this.getAddressType(address)]
      if (plugin) return plugin.messageHandler;
    }

    return this.unroutableMessageHandler
  }

  registerPlugin(addressType: AddressType, plugin: AddressTypePlugin) {
    this.plugins[addressType] = plugin;
  }

  unregisterPlugin(addressType: AddressType) {
    delete this.plugins[addressType];
  }

  convertAddressToString(address: Address) {
    let plugin = this.plugins[this.getAddressType(address)]
    return plugin ? plugin.convertAddressToString(address) : ''
  }

  convertAddressToUint8Array(address: Address) {
    let plugin = this.plugins[this.getAddressType(address)]
    return plugin ? plugin.convertAddressToUint8Array(address) : ''
  }
}
