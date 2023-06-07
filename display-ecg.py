import csv
import matplotlib.pyplot as plt
import sys

try:
    ecg_file_path = sys.argv[1]
except IndexError:
    ecg_file_path = "/Users/rcullen/Documents/ecg.csv"

with open(ecg_file_path) as f:
    reader = csv.reader(f)
    # skip metadata and header
    next(reader)
    next(reader)
    x = []
    y = []
    for line in reader:
        x.append(int(line[0]))
        y.append(int(line[1]))


plt.plot(x, y, label="$ECG (µV)$")
plt.legend()
plt.title("ECG over time")
plt.xlabel("Timestamp")
plt.ylabel("ECG $(µV)$")
plt.show()
