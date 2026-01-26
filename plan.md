# AxKeyStore

## Flow
1. AxKeyStore is an open source command line tool which stores keys and passwords securely in the user's GitHub repo.

2. User should setup a private GitHub repo to store the keys and passwords.

3. AxKeyStore authenticates user using their GitHub OAUTH credentials.

4. Once the authentication is through, user should give write access to the already setup private GitHub repo to AxKeyStore application.

5. Once the application receives the write access, the application is ready to store the keys and passwords.

6. User can use the axkeystore command to store the keys and passwords.

7. User can add key values, get key values, update key values and delete key values.

8. User can also add categories to organize the keys and passwords.

9. if no value is given for a key, generate a alpha numeric value with only alphabets. Max length shall be 36 characters. Minimum shall be 6. Then show that value to the user and ask for confirmation.

### Master Password management
1. User can set a master password for the application.

2. User cannot remove the master password for the application.

3. If master password is not set, user shall be asked to set it.

4. If master password is set, user shall be asked to enter it before storing or retrieving any key.

5. If master password is set, user shall be asked to enter it before updating or deleting any key.

6. The master password is used to encrypt a 36 character long random string. This encrypted string is called master key.

7. Master key shall be stored in github private repo in encrypted form. 

8. The master key shall be used to encrypt the actual key values.

