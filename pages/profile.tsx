import {Avatar, Card, Button, Center, Group, Loader, Paper, Stack, Text, Title} from "@mantine/core";
import {appwrite} from "../utils/appwrite_utils";
import React, {useEffect, useState} from "react";
import {NextRouter, useRouter} from "next/router";
import {Models} from "appwrite";
import {Reservation} from "../model/Reservation";
import ReservationComponent from "../components/Reservation";
import GameTable from "../model/GameTable";

interface IParams {
    gameTables: GameTable[]
}

function signOut(router: NextRouter) {
    const promise = appwrite.account.deleteSession('current');

    promise.then(function (response) {
        localStorage.removeItem('auth_state');
        router.push("/signin")
        console.log(response); // Success
    }, function (error) {
        console.log('Auth', error);
        console.log(error); // Failure
    });
}

export default function Profile(params: IParams) {
    const router = useRouter()

    const [loading, setLoading] = useState(true)
    const [user, setUser] = useState<Models.User<Models.Preferences>>(null)
    const [reservations, setReservations] = useState<Reservation[]>([])

    useEffect(() => {
        function refetchReservations() {
            appwrite.database.listDocuments<Reservation>('62cdcab0e527f917eb34').then((res) => {
                setReservations(res.documents)
                console.log("Reservation updated")
            });
        }

        if (user == null) {
            appwrite.account.get()
                .then((account) => {
                    setUser(account);
                    setLoading(false);
                })
                .catch(() => {
                    setUser(null)
                    setLoading(false);
                    router.push("/signin")
                })
        } else {
            refetchReservations();

            return appwrite.client.subscribe('collections.62cdcab0e527f917eb34.documents', response => {
                refetchReservations()
                console.log(response);
            });
        }
    }, [router, user]);

    if (loading)
        return <Center> <Loader/> </Center>;

    if (user == null)
        return (<></>)

    return (<>
        <Paper shadow={"md"} p={"xl"} sx={(theme) => ({
            backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[1],
            margin: theme.spacing.md
        })}>

            <Group position={"apart"}>
                <Group noWrap={true}>
                    <Avatar src={appwrite.avatars.getInitials(user.name, 64, 64).toString()} radius={"md"}/>

                    <Text size="md">{user.name}</Text>
                </Group>

                <Button variant={"filled"} color={'red'} onClick={() => signOut(router)}>Sign out</Button>
            </Group>

        </Paper>

        <Stack p={"xl"}>
            <Title order={2}>Rezervările tale:</Title>

            {reservations.length == 0 &&
                <Text size={"lg"}>Nu ați făcut nici o rezervare</Text>
            }

            {reservations.map((reservation) => (
                <Card key={reservation.$id}>
                    {ReservationComponent(
                            reservation,
                            params.gameTables.find((element) => element.$id == reservation.table_id),
                            true)}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const gameTables = await appwrite.database.listDocuments<GameTable>("62cdcac1bb2c8a4e5e48")

    const props: IParams = {
        gameTables: gameTables.documents
    }

    return {
        props: props
    }
}
