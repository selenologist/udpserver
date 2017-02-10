import socket
import threading
import binascii

# Create a UDP socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

server_address = ('127.0.0.1', 30000)

def recv_loop(sock):
    print("Started recv thread ", sock)
    while True:
        (data, address) = sock.recvfrom(65536)
        print("\r[Message from %s:] %s\nhex? " %(address, binascii.hexlify(data)))


def send_packet(sock, hexstring):
    data    = binascii.unhexlify(hexstring)
    sock.sendto(data, server_address)


recv_thread = threading.Thread(target=lambda: recv_loop(sock))
recv_thread.start()

while True:
    hexstring = input("hex? ")
    send_packet(sock, hexstring)

recv_thread.join()
    
