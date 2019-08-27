import os, datetime, time
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
    working_dir = os.getcwd() + "/../../"
    server_dir = working_dir + "server/"
    client_dir = working_dir + "cli/"
    conf_dir = working_dir + "test/conf/"

    environment = {
        "working_dir": working_dir,
        "server": { 
            "bin": "rustbolt",
            "root": server_dir,
            "test": "{}target/{}/".format(server_dir,test_version) },
        "cli": {
            "bin": "rbcli",
            "root": client_dir,
            "test": "{}target/{}/".format(client_dir,test_version)
        },
        "conf": {
            "root" : conf_dir,
            "server": { 
                "dir": "{}server/".format(conf_dir),
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

def build(project, version, env):
    print_info("building {} version: {}".format(project, version))
    os.chdir(env[project]["root"])

    if subprocess.run(["cargo", "build"]).returncode != 0:
        return print_error("build Error")
    print_pass("build success, {} is ready".format(project))
    return env[project]["test"]

def run_server(server_id, build_dir, version, env):
    server_bin =  build_dir + env["server"]["bin"] 

    # Copy configuration files
    print_info("copying configuration files")
    conf = "{}{}/".format(env["conf"]["server"]["dir"],server_id)
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
    print_exec("rbcli {}".format(" ".join(cmd)))
    cli_bin =  build_dir + env["cli"]["bin"] 
    return json.loads(subprocess.check_output([cli_bin, "-j"] + cmd).decode('ascii'))

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

def test():
    env = get_env("debug")

    # Build Lightning Server
    server_build_dir = build("server", "debug", env)
    
    # Build Cli
    cli_build_dir = build("cli", "debug", env)

    # wipe data
    data_dir = server_build_dir + "ln"
    print_info("wiping data: {}".format(data_dir))
    subprocess.run(["rm", "-rf", data_dir])  

    # Establish Bitcoind RPC
    bitcoin_cli = BitcoinClient("admin1:123@127.0.0.1:19001")
    info = bitcoin_cli.req("getblockchaininfo", [])
    print_info("current block height: {}".format(info["result"]["blocks"]))
    print_info("best block hash: {}".format(info["result"]["bestblockhash"]))

    # Run Server
    s1 = run_server(1, server_build_dir, "debug", env)
    s2 = run_server(2, server_build_dir, "debug", env)
    sleep("wait to stablize", 5)

    """
    ██╗███╗   ██╗███████╗ ██████╗ 
    ██║████╗  ██║██╔════╝██╔═══██╗
    ██║██╔██╗ ██║█████╗  ██║   ██║
    ██║██║╚██╗██║██╔══╝  ██║   ██║
    ██║██║ ╚████║██║     ╚██████╔╝
    ╚═╝╚═╝  ╚═══╝╚═╝      ╚═════╝ 
    """
    r0 = run_cli(cli_build_dir, env, ["info", "-a"])
    print_pass("got node #1 addresses: {}".format(r0))

    r01 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "info", "-a"])
    print_pass("got node #2 addresses: {}".format(r01))

    addrs = r0['imported_addresses'] + r01['imported_addresses']
    for addr in addrs:
        fund(addr, 0.1, bitcoin_cli)

    sleep("generate blocks", 5)
    gen = bitcoin_cli.req("generate", [10])
    print_info(json.dumps(gen, indent=4, sort_keys=True))
    sleep("wait to stablize", 5)

    r1 = run_cli(cli_build_dir, env, ["info", "-n"])
    print_pass("got node #1 public key: {}".format(r1["node_id"]))

    r2 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "info", "-n"])
    print_pass("got node #2 public key: {}".format(r2["node_id"]))

    """
    ██████╗ ███████╗███████╗██████╗ 
    ██╔══██╗██╔════╝██╔════╝██╔══██╗
    ██████╔╝█████╗  █████╗  ██████╔╝
    ██╔═══╝ ██╔══╝  ██╔══╝  ██╔══██╗
    ██║     ███████╗███████╗██║  ██║
    ╚═╝     ╚══════╝╚══════╝╚═╝  ╚═╝
    """
    r3 = run_cli(cli_build_dir, env, ["peer", "-c", "{}@{}:{}".format(r2["node_id"], "127.0.0.1", "9736")])
    print_pass("got connection: {}".format(r3))

    sleep("wait to establish connection", 5)
    r4 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "peer", "-l"])
    print_pass("got node #2 peers: {}".format(r4))

    """
     ██████╗██╗  ██╗ █████╗ ███╗   ██╗███╗   ██╗███████╗██╗      
    ██╔════╝██║  ██║██╔══██╗████╗  ██║████╗  ██║██╔════╝██║     
    ██║     ███████║███████║██╔██╗ ██║██╔██╗ ██║█████╗  ██║     
    ██║     ██╔══██║██╔══██║██║╚██╗██║██║╚██╗██║██╔══╝  ██║     
    ╚██████╗██║  ██║██║  ██║██║ ╚████║██║ ╚████║███████╗███████╗
     ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═══╝╚══════╝╚══════╝
    """
    r5 = run_cli(cli_build_dir, env, ["channel", "-c", r2["node_id"], "1000000", "50"])
    print_pass("got channel: {}".format(r5))

    sleep("generate blocks", 5)
    gen = bitcoin_cli.req("generate", [10])
    print_info(json.dumps(gen, indent=4, sort_keys=True))
    sleep("wait to stablize", 5)

    r6 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
    print_pass("got channel list: {}".format(r6))

    r7 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
    print_pass("got channel list node #2: {}".format(r7))

    r61 = run_cli(cli_build_dir, env, ["channel", "-l", "live"])
    print_pass("got channel list: {}".format(r61))

    # r8 = run_cli(cli_build_dir, env, ["channel", "-k", r6["channels"][0]["id"]])
    # print_pass("channel killed: {}".format(r8))

    # r9 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
    # print_pass("got channel list: {}".format(r9))

    # r10 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-c", r2["node_id"], "100000", "5000"])
    # print_pass("got channel: {}".format(r10))

    # r11 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
    # print_pass("got channel list node #2: {}".format(r11))

    # r12 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-x"])
    # print_pass("channel killall executed node #2: {}".format(r12))

    # r13 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
    # print_pass("got channel list node #2: {}".format(r13))

    # r14 = run_cli(cli_build_dir, env, ["channel", "-c", r1["node_id"], "100000", "5000"])
    # print_pass("got channel: {}".format(r14))

    # sleep("generate blocks", 5)
    # gen = bitcoin_cli.req("generate", [10])
    # print_info(json.dumps(gen, indent=4, sort_keys=True))
    # sleep("wait to stablize", 5)

    """ 
    ██╗███╗   ██╗██╗   ██╗ ██████╗ ██╗ ██████╗███████╗
    ██║████╗  ██║██║   ██║██╔═══██╗██║██╔════╝██╔════╝
    ██║██╔██╗ ██║██║   ██║██║   ██║██║██║     █████╗  
    ██║██║╚██╗██║╚██╗ ██╔╝██║   ██║██║██║     ██╔══╝  
    ██║██║ ╚████║ ╚████╔╝ ╚██████╔╝██║╚██████╗███████╗
    ╚═╝╚═╝  ╚═══╝  ╚═══╝   ╚═════╝ ╚═╝ ╚═════╝╚══════╝
    """
    # r140 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
    # print_pass("got channel list node #2: {}".format(r140))

    # r141 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
    # print_pass("got channel list node #2: {}".format(r141))

    # r142 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "live"])
    # print_pass("got channel list node #2: {}".format(r142))

    # r143 = run_cli(cli_build_dir, env, ["channel", "-l", "live"])
    # print_pass("got channel list node #2: {}".format(r143))

    r15 = run_cli(cli_build_dir, env, ["invoice", "-c", "10020000"])
    print_pass("got invoice: {}".format(r15))

    sleep("generate blocks", 5)
    gen = bitcoin_cli.req("generate", [10])
    print_info(json.dumps(gen, indent=4, sort_keys=True))
    sleep("wait to stablize", 5)

    r151 = run_cli(cli_build_dir, env, ["channel", "-l", "all"])
    print_pass("got channel list: {}".format(r151))

    r152 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l", "all"])
    print_pass("got channel list node #2: {}".format(r152))

    r16 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "invoice", "-p", r15["invoice"]])
    print_error("pay invoice: {}".format(r16))

    sleep("generate blocks", 5)
    gen = bitcoin_cli.req("generate", [10])
    print_info(json.dumps(gen, indent=4, sort_keys=True))
    sleep("shut down", 5)

    s1.kill()
    s2.kill()

    # wipe data
    data_dir = server_build_dir + "ln"
    print_info("wiping data: {}".format(data_dir))
    subprocess.run(["rm", "-rf", data_dir])  


if __name__ == '__main__':
    test()
