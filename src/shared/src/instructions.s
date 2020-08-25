.intel_syntax noprefix

iret:
  mov ebp, esp

  push dword PTR [ebp+4]
  push dword PTR [ebp+8]
  push dword PTR [ebp+12]  
  push dword PTR [ebp+16]
  push dword PTR [ebp+20]
  iret