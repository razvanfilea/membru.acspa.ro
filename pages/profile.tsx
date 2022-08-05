import {Button, Card, Center, Group, Loader, Paper, Stack, Text, Title} from "@mantine/core";
import React, {useEffect, useState} from "react";
import {NextRouter, useRouter} from "next/router";
import ReservationComponent from "../components/Reservation";
import {useAuth} from "../components/AuthProvider";
import {supabase} from "../utils/supabase_utils";
import {GameTable, Reservation} from "../types/wrapper";

interface IParams {
    gameTables: GameTable[]
}

async function signOut(router: NextRouter) {
    const {error} = await supabase.auth.signOut()

    if (error == null) {
        await router.push("/signin")
    } else {
        console.log('Auth', error);
        console.log(error); // Failure
    }
}

export default function Profile(params: IParams) {
    const router = useRouter()
    const auth = useAuth()

    const [reservations, setReservations] = useState<Reservation[]>([])

    useEffect(() => {
        function refetchReservations() {
            supabase.from<Reservation>('rezervari')
                .select('*')
                .then(value => {
                    if (value.data != null) {
                        setReservations(value.data)
                        console.log("Reservations updated")
                    }
                })
        }

        if (!auth.loading) {
            if (auth.user != null) {
                refetchReservations();

                const subscription = supabase.from<Reservation>('rezervari')
                    .on('*', payload => {
                        console.log(payload)
                        refetchReservations()
                    })
                    .subscribe()

                return () => {
                    subscription?.unsubscribe()
                }
            } else {
                router.push('/signin')
            }
        }
    }, [auth, router]);

    if (auth.loading)
        return <Center> <Loader/> </Center>;

    if (auth.user == null)
        return (<></>)

    return (<>
        <Paper shadow={"md"} p={"xl"} sx={(theme) => ({
            backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[1],
            margin: theme.spacing.md
        })}>

            <Group position={"apart"}>
                <Group noWrap={true}>
                    {/*TODO <Avatar src={appwrite.avatars.getInitials(auth.profile.name, 64, 64).toString()} radius={"md"}/>
*/}
                    {auth.profile != null &&
                        <Text size="md">{auth.profile.name}</Text>
                    }
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
                <Card key={reservation.id}>
                    {ReservationComponent(
                        reservation,
                        params.gameTables.find((element) => element.id == reservation.table_id),
                        true)}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const {data: gameTables} = await supabase.from<GameTable>('mese').select('*')

    const props: IParams = {
        gameTables
    }

    return {
        props: props
    }
}
