import os, datetime, time, unittest, signal
import subprocess, json
import requests
from requests.auth import HTTPBasicAuth

# Print Messages
def get_now():
    now = datetime.datetime.utcnow()
    return now.strftime('%H:%M:%S')

def print_info(message):
    print("{} \x1b[1;34m[ INFO]\x1b[0m {} ... ".format(get_now(), message))

def print_exec(message):
    print("{} \x1b[1;33m[ >>>>]\x1b[0m {}".format(get_now(), message))

def print_pass(message):
    print("{} \x1b[1;32m[ PASS]\x1b[0m {} ... ".format(get_now(), message))

def print_bold(message, tag):
    print("{} \x1b[1;37m[{}]\x1b[0m {} ... ".format(get_now(), tag.upper(), message))

def print_error(message):
    print("{} \x1b[1;31m[ERROR]\x1b[0m {} ... ".format(get_now(), message))

def get_env(test_version):
    working_dir = os.getenv("WORKING_DIR")
    running_env = os.getenv("RUNNING_ENV")
    travis_job_id = os.getenv("TRAVIS_JOB_ID")
    home = os.getenv("HOME")
    host = os.getenv("HOST")
    bitcoind_host = os.getenv("BITCOIND_HOST")

    if host is None:
        host = "lightning"

    if working_dir is None:
        working_dir = "/lightning/"

    if running_env is None:
        running_env = "docker"

    if bitcoind_host is None:
        bitcoind_host = "regtest-0"

    server_dir = working_dir + "server/"
    client_dir = working_dir + "cli/"
    conf_dir = working_dir + "test/conf/"

    environment = {
        "working_dir": working_dir,
        "travis_job_id": travis_job_id,
        "host": host,
        "bitcoind_host": bitcoind_host,
        "home": home,
        "server": {
            "bin": "rustbolt",
            "root": server_dir,
            "test": "{}target/{}/".format(server_dir,test_version),
        },
        "cli": {
            "bin": "rbcli",
            "root": client_dir,
            "test": "{}target/{}/".format(client_dir,test_version),
        },
        "conf": {
            "root" : conf_dir,
            "server": {
                "dir": "{}{}/".format(conf_dir, running_env),
                "ln": "ln.conf.toml",
                "node": "node.conf.toml"
            }
        }
    }
    return environment

def sleep(action, secs):
    print_bold("{} in next {} sec(s)".format(action, secs), " warn")
    end = ""
    for i in range(0, secs):
        if i + 1 == secs:
            end = "\n"
        if i:
            print("{}...".format(secs-i), end=end, flush=True )
            time.sleep(1)
    return

def run_server(server_id, build_dir, version, env):
    server_bin =  build_dir + env["server"]["bin"]

    # Copy configuration files
    print_info("copying configuration files")
    conf = "{}{}/".format(env["conf"]["server"]["dir"],server_id)
    subprocess.run(["rm", "-rf", "{}{}".format(build_dir, server_id)])
    subprocess.run(["cp", "-r", conf, "{}{}".format(build_dir, server_id)])

    data = build_dir + "ln/data_{}/".format(server_id)

    print_info("preparing local storage files: {}".format(data))

    # Create data storage folder
    subprocess.run(["mkdir", "-p", data])
    os.chdir(build_dir)

    # Run server
    print_info("run: { " + server_bin + " }")
    server = subprocess.Popen([
        server_bin,
        "{}{}/{}".format(build_dir, server_id, env["conf"]["server"]["ln"]),
        "{}{}/{}".format(build_dir, server_id, env["conf"]["server"]["node"]),
    ])

    if server.returncode != 0 and server.returncode != None:
        return print_error("server run error")

    print_pass("servre running:[PID]:{} [SERVER_ID]:{}".format(
        str(server.pid),
        server_id
    ))

    return server

def run_cli(build_dir, env, cmd):
    print_exec("kcov --exclude-pattern=/.cargo,/usr/lib {}coverage/ rbcli {}".format(env["working_dir"], " ".join(cmd)))
    cli_bin =  build_dir + env["cli"]["bin"]
    return json.loads(subprocess.check_output([
        "{}/.cargo/bin/kcov", 
        "--coveralls-id={}".format(env["travis_job_id"]),
        "--exclude-pattern=/.cargo,/usr/lib ",
        "{}coverage/".format(env["working_dir"]), 
        cli_bin, 
        "-j"
    ] + cmd).decode('ascii'))

def fund(addr, amount, cli):
    res = cli.req("sendtoaddress", [addr, amount])
    print_info("funded {}BTC to {}, tx_id: {}".format(amount, addr, res['result']))


class BitcoinClient:
    def __init__(self, rpc_url):
        (credential, rpc_url) = rpc_url.split("@")
        (usr, pwd) = credential.split(":")
        self.rpc_url = "http://{}".format(rpc_url)
        self.headers = {'content-type': 'application/json'}
        self.req_id = 0
        self.auth=HTTPBasicAuth(usr, pwd)
        self.payload = {
            "method": "",
            "params": [],
            "jsonrpc": "2.0",
            "id": self.req_id,
        }

    def raw_request(self, url, data, headers):
        self.req_id += 1
        return requests.post(url, data=data, headers=headers, auth=self.auth).json()

    def req(self, method, params):
        self.payload["method"] = method
        self.payload["params"] = params
        return self.raw_request(self.rpc_url, data=json.dumps(self.payload), headers=self.headers)


# not a real unittest, only for count cases
class TestCases(unittest.TestCase):
    @classmethod
    def setUpClass(self):
        self.env = get_env("debug")
        self.client = BitcoinClient("admin1:123@{}:19001".format(self.env["bitcoind_host"]))

        print_info("Initial Start")
        self.server_build_dir = self.env["server"]["test"]
        self.cli_build_dir = self.env["cli"]["test"]

        data_dir = self.server_build_dir + "ln"
        print_info("wiping data: {}".format(data_dir))
        subprocess.run(["rm", "-rf", data_dir])

        sleep("wait for node initialize...", 10)
        self.ln_node_1 = run_server(1, self.server_build_dir, "debug", self.env)
        self.ln_node_2 = run_server(2, self.server_build_dir, "debug", self.env)

        print_info("Shut Down")
        sleep("wait for a long time", 5)
        self.client.req("generate", [200])
        node_1 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "info", "-a"])
        node_2 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8124".format(self.env["host"]), "info", "-a"])
        addrs = node_1['imported_addresses'] + node_2['imported_addresses']
        for addr in addrs:
            fund(addr, 0.5, self.client)
        self.client.req("generate", [10])
        sleep("wait generate", 10)
        self.ln_node_1.kill()
        self.ln_node_2.kill()
        # wipe data
        if self.server_build_dir:
            data_dir = self.server_build_dir + "ln"
            print_info("wiping data: {}".format(data_dir))
            subprocess.run(["rm", "-rf", data_dir])
        sleep("wait kill", 5)

        print_info("Restart")
        self.server_build_dir = self.env["server"]["test"]
        self.cli_build_dir = self.env["cli"]["test"]

        data_dir = self.server_build_dir + "ln"
        print_info("wiping data: {}".format(data_dir))
        subprocess.run(["rm", "-rf", data_dir])

        sleep("wait for node initialize...", 10)
        self.ln_node_1 = run_server(1, self.server_build_dir, "debug", self.env)
        self.ln_node_2 = run_server(2, self.server_build_dir, "debug", self.env)

        def do_when_kill(signal, frame):
            self.tearDownClass(self)
            raise KeyboardInterrupt
        signal.signal(signal.SIGINT, do_when_kill)
        return
    @classmethod
    def tearDownClass(self):
        self.ln_node_1.kill()
        self.ln_node_2.kill()
        # wipe data
        if self.server_build_dir:
            data_dir = self.server_build_dir + "ln"
            print_info("wiping data: {}".format(data_dir))
            subprocess.run(["rm", "-rf", data_dir])
        return
    def setUp(self):
        sleep("wait to stablize", 5)
        return
    def tearDown(self):
        return

    def test_0_bitcoind_client(self):
        print_info("checking node_0 bitcoind")
        info = self.client.req("getblockchaininfo", [])
        print_info(info)
        self.assertIsNone(info["error"], "failed to get blockchain info {}".format(info))
        return

    def generate_block(self, nums=1):
        self.client.req("generate", [nums])
        sleep("generate blocks", 5)
        return
    def test_1_info_node(self):
        node_1 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "info", "-a"])
        self.assertEqual(len(node_1["imported_addresses"]), 2, "imported error")
        print_pass("got node #1 addresses: {}".format(node_1))
        node_2 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8124".format(self.env["host"]), "info", "-a"])
        self.assertEqual(len(node_2["imported_addresses"]), 2, "imported error")
        print_pass("got node #2 addresses: {}".format(node_2))
        addrs = node_1['imported_addresses'] + node_2['imported_addresses']
        for addr in addrs:
            fund(addr, 0.5, self.client)
        self.generate_block(10)
        return
    def test_1_info_pubkey(self):
        node_1 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "info", "-n"])
        print_pass("got node #1 public key: {}".format(node_1["node_id"]))
        self.assertIsNotNone(node_1["node_id"])
        self.__class__.node_id_1 = node_1["node_id"]
        node_2 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8124".format(self.env["host"]), "info", "-n"])
        print_pass("got node #2 public key: {}".format(node_2["node_id"]))
        self.assertIsNotNone(node_2["node_id"])
        self.__class__.node_id_2 = node_2["node_id"]
        return
    def test_2_0_peer_connect(self):
        connect = run_cli(
            self.cli_build_dir, self.env,
            ["-n", "{}:8123".format(self.env["host"]), "peer", "-c", "{}@{}:{}".format(self.node_id_2, self.env["host"], "9736")]
        )
        print_pass("got connection: {}".format(connect))
        self.assertIsNotNone(connect["response"])
        r4 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "peer", "-l"])
        print_pass("got node #1 peers: {}".format(r4))
        r41 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8124".format(self.env["host"]), "peer", "-l"])
        print_pass("got node #2 peers: {}".format(r41))
        return
    def test_2_1_peers(self):
        r4 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8124".format(self.env["host"]), "peer", "-l"])
        print_pass("got node #2 peers: {}".format(r4))
        self.assertTrue(len(r4["peers"]) > 0)
        return
    def test_3_0_channel_connect(self):
        r5 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "channel", "-c", self.node_id_2, "2000000", "100500000"])
        print_pass("got channel: {}".format(r5))
        self.assertIsNotNone(r5["channel"])
        sleep("generate blocks", 5)
        self.generate_block(10)
        return
    def test_3_1_channel_list(self):
        r6 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "channel", "-l", "all"])
        print_pass("got channel list: {}".format(r6))
        self.assertTrue(len(r6["channels"]) > 0)
        return
    def test_3_2_node_2_channel_list(self):
        r7 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8124".format(self.env["host"]), "channel", "-l", "all"])
        print_pass("got channel list node #2: {}".format(r7))
        self.assertTrue(len(r7["channels"]) > 0)
        return
    def test_3_3_channel_not_live(self):
        r61 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "channel", "-l", "live"])
        print_pass("got channel list: {}".format(r61))
        # self.assertTrue(len(r61["channels"]) == 0)
        return
    def test_4_0_invoce(self):
        r15 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "invoice", "-c", "1001000"])
        print_pass("got invoice: {}".format(r15))
        self.assertTrue("error" not in r15)
        r151 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "channel", "-l", "all"])
        print_pass("got channel list: {}".format(r151))
        self.assertTrue("error" not in r151)
        r152 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8124".format(self.env["host"]), "channel", "-l", "all"])
        print_pass("got channel list node #2: {}".format(r152))
        self.assertTrue("error" not in r152)
        self.generate_block(10)
        r16 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8124".format(self.env["host"]), "invoice", "-p", r15["invoice"]])
        print_info("pay invoice: {}".format(r16))
        self.assertTrue("error" not in r16)
        return
    # def test_4_1_check_channel(self):
    #     return
    def test_5_0_kill_channel(self):
        r6 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "channel", "-l", "all"])
        print_pass("got channel list: {}".format(r6))
        r8 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "channel", "-k", r6["channels"][0]["id"]])
        print_pass("channel killed: {}".format(r8))
        r9 = run_cli(self.cli_build_dir, self.env, ["-n", "{}:8123".format(self.env["host"]), "channel", "-l", "all"])
        print_pass("got channel list: {}".format(r9))
        self.assertTrue("error" not in r8)
        return

# def test():
#     env = get_env("debug")

#     # Build Lightning Server
#     server_build_dir = build("server", "debug", env)

#     # Build Cli
#     cli_build_dir = build("cli", "debug", env)

#     # wipe data
#     data_dir = server_build_dir + "ln"
#     print_info("wiping data: {}".format(data_dir))
#     subprocess.run(["rm", "-rf", data_dir])

#     # Establish Bitcoind RPC
#     bitcoin_cli = BitcoinClient("admin1:123@127.0.0.1:19011")
#     info = bitcoin_cli.req("getblockchaininfo", [])
#     print_info("current block height: {}".format(info["result"]["blocks"]))
#     print_info("best block hash: {}".format(info["result"]["bestblockhash"]))

#     # Run Server
#     s1 = run_server(1, server_build_dir, "debug", env)
#     s2 = run_server(2, server_build_dir, "debug", env)
#     sleep("wait to stablize", 5)

#     """
#     ██╗███╗   ██╗███████╗ ██████╗
#     ██║████╗  ██║██╔════╝██╔═══██╗
#     ██║██╔██╗ ██║█████╗  ██║   ██║
#     ██║██║╚██╗██║██╔══╝  ██║   ██║
#     ██║██║ ╚████║██║     ╚██████╔╝
#     ╚═╝╚═╝  ╚═══╝╚═╝      ╚═════╝
#     """
#     r0 = run_cli(cli_build_dir, env, ["info", "-a"])
#     print_pass("got node #1 addresses: {}".format(r0))

#     r01 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "info", "-a"])
#     print_pass("got node #2 addresses: {}".format(r01))

#     addrs = r0['imported_addresses'] + r01['imported_addresses']
#     for addr in addrs:
#         fund(addr, 0.5, bitcoin_cli)

#     sleep("generate blocks", 5)
#     gen = bitcoin_cli.req("generate", [10])
#     sleep("wait to stablize", 5)

#     r1 = run_cli(cli_build_dir, env, ["info", "-n"])
#     print_pass("got node #1 public key: {}".format(r1["node_id"]))

#     r2 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "info", "-n"])
#     print_pass("got node #2 public key: {}".format(r2["node_id"]))

#     """
#     ██████╗ ███████╗███████╗██████╗
#     ██╔══██╗██╔════╝██╔════╝██╔══██╗
#     ██████╔╝█████╗  █████╗  ██████╔╝
#     ██╔═══╝ ██╔══╝  ██╔══╝  ██╔══██╗
#     ██║     ███████╗███████╗██║  ██║
#     ╚═╝     ╚══════╝╚══════╝╚═╝  ╚═╝
#     """
#     r3 = run_cli(cli_build_dir, env, ["peer", "-c", "{}@{}:{}".format(r2["node_id"], "127.0.0.1", "9736")])
#     print_pass("got connection: {}".format(r3))

#     sleep("wait to establish connection", 5)
#     r4 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "peer", "-l"])
#     print_pass("got node #2 peers: {}".format(r4))

#     """
#      ██████╗██╗  ██╗ █████╗ ███╗   ██╗███╗   ██╗███████╗██╗
#     ██╔════╝██║  ██║██╔══██╗████╗  ██║████╗  ██║██╔════╝██║
#     ██║     ███████║███████║██╔██╗ ██║██╔██╗ ██║█████╗  ██║
#     ██║     ██╔══██║██╔══██║██║╚██╗██║██║╚██╗██║██╔══╝  ██║
#     ╚██████╗██║  ██║██║  ██║██║ ╚████║██║ ╚████║███████╗███████╗
#      ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═══╝╚══════╝╚══════╝
#     """
#     r5 = run_cli(cli_build_dir, env, ["channel", "-c", r2["node_id"], "2000000", "100500000"])
#     print_pass("got channel: {}".format(r5))

#     sleep("generate blocks", 5)
#     gen = bitcoin_cli.req("generate", [10])
#     sleep("wait to stablize", 5)

#     r6 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
#     print_pass("got channel list: {}".format(r6))

#     r7 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
#     print_pass("got channel list node #2: {}".format(r7))

#     r61 = run_cli(cli_build_dir, env, ["channel", "-l", "live"])
#     print_pass("got channel list: {}".format(r61))

#     # r8 = run_cli(cli_build_dir, env, ["channel", "-k", r6["channels"][0]["id"]])
#     # print_pass("channel killed: {}".format(r8))

#     # r9 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
#     # print_pass("got channel list: {}".format(r9))

#     # r10 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-c", r2["node_id"], "100000", "5000"])
#     # print_pass("got channel: {}".format(r10))

#     # r11 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
#     # print_pass("got channel list node #2: {}".format(r11))

#     # r12 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-x"])
#     # print_pass("channel killall executed node #2: {}".format(r12))

#     # r13 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
#     # print_pass("got channel list node #2: {}".format(r13))

#     # r14 = run_cli(cli_build_dir, env, ["channel", "-c", r1["node_id"], "100000", "5000"])
#     # print_pass("got channel: {}".format(r14))

#     # sleep("generate blocks", 5)
#     # gen = bitcoin_cli.req("generate", [10])
#     # print_info(json.dumps(gen, indent=4, sort_keys=True))
#     # sleep("wait to stablize", 5)

#     """
#     ██╗███╗   ██╗██╗   ██╗ ██████╗ ██╗ ██████╗███████╗
#     ██║████╗  ██║██║   ██║██╔═══██╗██║██╔════╝██╔════╝
#     ██║██╔██╗ ██║██║   ██║██║   ██║██║██║     █████╗
#     ██║██║╚██╗██║╚██╗ ██╔╝██║   ██║██║██║     ██╔══╝
#     ██║██║ ╚████║ ╚████╔╝ ╚██████╔╝██║╚██████╗███████╗
#     ╚═╝╚═╝  ╚═══╝  ╚═══╝   ╚═════╝ ╚═╝ ╚═════╝╚══════╝
#     """
#     # r140 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
#     # print_pass("got channel list node #2: {}".format(r140))

#     # r141 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
#     # print_pass("got channel list node #2: {}".format(r141))

#     # r142 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "live"])
#     # print_pass("got channel list node #2: {}".format(r142))

#     # r143 = run_cli(cli_build_dir, env, ["channel", "-l", "live"])
#     # print_pass("got channel list node #2: {}".format(r143))

#     # Create Invoice: 1001 msat, which is 1.001 sat
#     r15 = run_cli(cli_build_dir, env, ["invoice", "-c", "1001000"])
#     print_pass("got invoice: {}".format(r15))

#     r151 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
#     print_pass("got channel list: {}".format(r151))

#     r152 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
#     print_pass("got channel list node #2: {}".format(r152))

#     r16 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "invoice", "-p", r15["invoice"]])
#     print_info("pay invoice: {}".format(r16))

#     sleep("close channel", 2)

#     r8 = run_cli(cli_build_dir, env, ["channel", "-k", r6["channels"][0]["id"]])
#     print_pass("channel killed: {}".format(r8))

#     sleep("generate blocks", 2)
#     gen = bitcoin_cli.req("generate", [10])

#     r9 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
#     print_pass("got channel list: {}".format(r9))

#     sleep("shut down", 5)

#     s1.kill()
#     s2.kill()

#     # wipe data
#     data_dir = server_build_dir + "ln"
#     print_info("wiping data: {}".format(data_dir))
#     subprocess.run(["rm", "-rf", data_dir])


if __name__ == '__main__':
    # test()
    unittest.main()
