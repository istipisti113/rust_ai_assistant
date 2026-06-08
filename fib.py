import sys

def fib(n):
    if n <= 1:
        return n
    else:
        return(fib(n-1) + fib(n-2))

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Please provide the number of Fibonacci terms as a command line argument.")
    else:
        n = int(sys.argv[1])
        print(fib(n))